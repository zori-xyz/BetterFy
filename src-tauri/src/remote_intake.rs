use crate::archive_inspector::{inspect_zip_bytes, ArchiveReport};
use crate::content_store::{self, RemotePackageSpec, MAX_ARTIFACT_BYTES};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT_ENCODING, CONTENT_LENGTH, LOCATION};
use reqwest::{StatusCode, Url};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_REDIRECTS: usize = 3;
const MAX_OPERATIONS: usize = 32;
static NEXT_OPERATION: AtomicU64 = AtomicU64::new(0);
static OPERATIONS: OnceLock<Mutex<BTreeMap<String, Arc<DownloadOperation>>>> = OnceLock::new();

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadPhase {
    Queued,
    Downloading,
    Verifying,
    Ready,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDownloadStatus {
    operation_id: String,
    package_id: String,
    phase: DownloadPhase,
    received_bytes: u64,
    expected_bytes: u64,
    content_identity: Option<String>,
    error_code: Option<String>,
    archive_report: Option<ArchiveReport>,
}

struct DownloadOperation {
    state: Mutex<ContentDownloadStatus>,
    cancelled: AtomicBool,
}

trait DownloadTransport: Send + Sync + 'static {
    fn fetch(
        &self,
        spec: &RemotePackageSpec,
        cancelled: &AtomicBool,
        on_chunk: &mut dyn FnMut(&[u8]) -> Result<(), String>,
    ) -> Result<(), String>;
}

/// Hosts a wardrobe archive's download may be redirected to. Hugging Face
/// answers a `resolve/...` URL with a redirect to a CDN host under
/// `cdn.hf.co` (observed once, from `us.aws.cdn.hf.co`, with a signed query
/// string); other regions are matched by the suffix and unverified. The
/// pinned SHA-256 decides whether the bytes are accepted, so the allowlist
/// only decides where BetterFy is willing to connect. The other mirror,
/// `raw.githubusercontent.com`, answered a `HEAD` request for an archive with
/// 200 directly on 2026-10-17 (no redirect), so it needs no entry here.
const WARDROBE_REDIRECT_SUFFIXES: &[&str] = &[".cdn.hf.co"];

/// HTTPS download with every address resolved, checked and pinned. By default
/// a redirect must stay on the first URL's origin. A transport made for a
/// CDN-backed source may also follow a redirect to a host under
/// `redirect_suffixes`, and only there may the URL carry a query string.
struct PinnedHttpsTransport {
    redirect_suffixes: &'static [&'static str],
    timeout: Duration,
}

impl PinnedHttpsTransport {
    const SAME_ORIGIN: Self = Self {
        redirect_suffixes: &[],
        timeout: Duration::from_secs(30),
    };
    /// A 24 MB archive needs more than the 30 seconds a small resource gets.
    const WARDROBE: Self = Self {
        redirect_suffixes: WARDROBE_REDIRECT_SUFFIXES,
        timeout: Duration::from_secs(600),
    };

    fn follows(&self, url: &Url) -> bool {
        url.host_str().is_some_and(|host| {
            self.redirect_suffixes
                .iter()
                .any(|suffix| host.len() > suffix.len() && host.ends_with(suffix))
        })
    }
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !(ip.is_unspecified()
                || ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_multicast()
                || ip.octets()[0] == 0
                || ip.octets()[0] >= 224
                || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
                || (ip.octets()[0] == 169 && ip.octets()[1] == 254))
        }
        IpAddr::V6(ip) => {
            !(ip.is_unspecified()
                || ip.is_loopback()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.is_multicast())
        }
    }
}

fn validate_url_with(url: &Url, allow_query: bool) -> Result<(&str, u16), String> {
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || (!allow_query && url.query().is_some())
    {
        return Err("download_url_invalid".to_string());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "download_url_invalid".to_string())?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "download_url_invalid".to_string())?;
    if port != 443 {
        return Err("download_url_invalid".to_string());
    }
    Ok((host, port))
}

fn resolve_public(host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|_| "download_dns_failed".to_string())?
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if addresses.is_empty() || addresses.iter().any(|address| !public_ip(address.ip())) {
        return Err("download_address_blocked".to_string());
    }
    Ok(addresses)
}

impl DownloadTransport for PinnedHttpsTransport {
    fn fetch(
        &self,
        spec: &RemotePackageSpec,
        cancelled: &AtomicBool,
        on_chunk: &mut dyn FnMut(&[u8]) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut url =
            Url::parse(&spec.download_url).map_err(|_| "download_url_invalid".to_string())?;
        let original_origin = url.origin().ascii_serialization();
        for redirect in 0..=MAX_REDIRECTS {
            if cancelled.load(Ordering::Relaxed) {
                return Err("download_cancelled".to_string());
            }
            let cross_origin = url.origin().ascii_serialization() != original_origin;
            if cross_origin && !self.follows(&url) {
                return Err("download_redirect_blocked".to_string());
            }
            let (host, port) = validate_url_with(&url, cross_origin)?;
            let addresses = resolve_public(host, port)?;
            let client = Client::builder()
                .https_only(true)
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(8))
                .timeout(self.timeout)
                .user_agent("BetterFy-content/1")
                .resolve_to_addrs(host, &addresses)
                .build()
                .map_err(|_| "download_transport_failed".to_string())?;
            let mut response = client
                .get(url.clone())
                .header(ACCEPT_ENCODING, "identity")
                .send()
                .map_err(|_| "download_transport_failed".to_string())?;
            let remote = response
                .remote_addr()
                .ok_or_else(|| "download_peer_unverified".to_string())?;
            if !public_ip(remote.ip()) || !addresses.iter().any(|item| item.ip() == remote.ip()) {
                return Err("download_peer_unverified".to_string());
            }
            if response.status().is_redirection() {
                if redirect == MAX_REDIRECTS {
                    return Err("download_redirect_blocked".to_string());
                }
                let location = response
                    .headers()
                    .get(LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| "download_redirect_blocked".to_string())?;
                url = url
                    .join(location)
                    .map_err(|_| "download_redirect_blocked".to_string())?;
                continue;
            }
            if response.status() != StatusCode::OK {
                return Err("download_http_failed".to_string());
            }
            if let Some(length) = response
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
            {
                if length != spec.size || length > MAX_ARTIFACT_BYTES {
                    return Err("download_size_mismatch".to_string());
                }
            }
            let mut buffer = [0u8; 32 * 1024];
            loop {
                if cancelled.load(Ordering::Relaxed) {
                    return Err("download_cancelled".to_string());
                }
                let read = response
                    .read(&mut buffer)
                    .map_err(|_| "download_transport_failed".to_string())?;
                if read == 0 {
                    break;
                }
                on_chunk(&buffer[..read])?;
            }
            return Ok(());
        }
        Err("download_redirect_blocked".to_string())
    }
}

fn fetch_verified_binary_with<T: DownloadTransport>(
    transport: &T,
    spec: &RemotePackageSpec,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, String> {
    if spec.size == 0 || spec.size > MAX_ARTIFACT_BYTES {
        return Err("download_size_mismatch".to_string());
    }
    let mut bytes = Vec::with_capacity(spec.size as usize);
    transport.fetch(spec, cancelled, &mut |chunk| {
        if bytes.len().saturating_add(chunk.len()) > spec.size as usize {
            return Err("download_size_mismatch".to_string());
        }
        bytes.extend_from_slice(chunk);
        Ok(())
    })?;
    if cancelled.load(Ordering::Relaxed) {
        return Err("download_cancelled".to_string());
    }
    if bytes.len() as u64 != spec.size || format!("{:x}", Sha256::digest(&bytes)) != spec.sha256 {
        return Err("download_hash_mismatch".to_string());
    }
    Ok(bytes)
}

/// Used only with URLs and hashes from validated package manifests (embedded
/// or from a signed catalog). No caller-supplied URL crosses the Tauri bridge.
pub(crate) fn fetch_pinned_pilot_resource(
    package_id: &str,
    resource: &crate::package_registry::Resource,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, String> {
    let download_url = crate::tree_pilot::pinned_url(package_id, resource)?;
    let spec = RemotePackageSpec {
        package_id: package_id.to_string(),
        version: crate::tree_pilot::source_commit(package_id)?,
        format: "raw".to_string(),
        file_name: "compiled-resource".to_string(),
        download_url,
        media_type: "application/octet-stream".to_string(),
        size: resource.bytes as u64,
        sha256: resource.sha256.to_string(),
    };
    fetch_verified_binary_with(&PinnedHttpsTransport::SAME_ORIGIN, &spec, cancelled)
}

/// Downloads a wardrobe item's pinned file (a ZIP, or a raw VPK). The URL and the expected size and
/// SHA-256 come from the embedded allowlist entry only.
pub(crate) fn fetch_pinned_wardrobe_archive(
    manifest: &crate::wardrobe::WardrobeManifest,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, String> {
    fetch_wardrobe_archive_with(&PinnedHttpsTransport::WARDROBE, manifest, cancelled)
}

/// Tries the manifest's mirrors in order and returns the first response whose
/// bytes are exactly the pinned ZIP. A mirror that does not have the file
/// (an HTTP error), cannot be reached, or serves other bytes (wrong size or
/// hash) is skipped; the download fails only when no mirror yields the pinned
/// bytes, with the last mirror's error. Cancelling stops at once. Every
/// attempt keeps the transport's own rules (HTTPS, public addresses, bounded
/// redirects), and no bytes from a skipped mirror are kept.
fn fetch_wardrobe_archive_with<T: DownloadTransport>(
    transport: &T,
    manifest: &crate::wardrobe::WardrobeManifest,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, String> {
    // A ZIP is inspected after the download; a raw VPK is pinned by its own
    // size and SHA-256 and checked by the analyzer on every build.
    let file_name = match manifest.archive.kind {
        crate::wardrobe::SourceKind::Zip => "wardrobe-archive.zip",
        crate::wardrobe::SourceKind::Vpk => "wardrobe-archive.vpk",
    };
    fetch_mirrors_with(
        transport,
        &manifest.id,
        &manifest.source.urls,
        manifest.archive.download_bytes(),
        manifest.archive.download_sha256(),
        file_name,
        cancelled,
    )
}

/// Downloads a map terrain's pinned ZIP from the mirrors of its allowlist
/// entry, with the same rules as a wardrobe archive.
pub(crate) fn fetch_pinned_map_archive(
    manifest: &crate::map_terrain::MapManifest,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, String> {
    fetch_mirrors_with(
        &PinnedHttpsTransport::WARDROBE,
        &manifest.id,
        &manifest.source.urls,
        manifest.archive.zip_bytes,
        &manifest.archive.zip_sha256,
        "map-archive.zip",
        cancelled,
    )
}

pub(crate) fn fetch_pinned_game_pack(
    manifest: &crate::game_packs::PackManifest,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, String> {
    fetch_mirrors_with(
        &PinnedHttpsTransport::WARDROBE,
        &manifest.id,
        &manifest.source.urls,
        manifest.archive.zip_bytes,
        &manifest.archive.zip_sha256,
        "game-pack.zip",
        cancelled,
    )
}

fn fetch_mirrors_with<T: DownloadTransport>(
    transport: &T,
    package_id: &str,
    urls: &[String],
    zip_bytes: u64,
    zip_sha256: &str,
    file_name: &str,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, String> {
    let mut last_error = "download_url_invalid".to_string();
    let (format, media_type) = if file_name.ends_with(".vpk") {
        ("raw", "application/octet-stream")
    } else {
        ("zip", "application/zip")
    };
    for url in urls {
        let spec = RemotePackageSpec {
            package_id: package_id.to_string(),
            version: "pinned-archive".to_string(),
            format: format.to_string(),
            file_name: file_name.to_string(),
            download_url: url.clone(),
            media_type: media_type.to_string(),
            size: zip_bytes,
            sha256: zip_sha256.to_string(),
        };
        match fetch_verified_binary_with(transport, &spec, cancelled) {
            Ok(bytes) => return Ok(bytes),
            Err(code) if code == "download_cancelled" => return Err(code),
            Err(code) => last_error = code,
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err("download_cancelled".to_string());
        }
    }
    Err(last_error)
}

fn operation_id() -> Result<String, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "download_unavailable".to_string())?
        .as_millis();
    Ok(format!(
        "download-{timestamp}-{}-{}",
        std::process::id(),
        NEXT_OPERATION.fetch_add(1, Ordering::Relaxed)
    ))
}

fn update(operation: &DownloadOperation, apply: impl FnOnce(&mut ContentDownloadStatus)) {
    if let Ok(mut state) = operation.state.lock() {
        apply(&mut state);
    }
}

fn unique_temporary(download_root: &Path, operation_id: &str) -> Result<PathBuf, String> {
    if !operation_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("download_operation_invalid".to_string());
    }
    Ok(download_root.join(format!(".{operation_id}.part")))
}

fn run_download<T: DownloadTransport>(
    app_data_root: &Path,
    spec: RemotePackageSpec,
    operation: &DownloadOperation,
    transport: &T,
) -> Result<(String, Option<ArchiveReport>), String> {
    let download_root = content_store::prepare_download_root(app_data_root)?;
    let operation_id = operation
        .state
        .lock()
        .map_err(|_| "download_unavailable".to_string())?
        .operation_id
        .clone();
    let temporary = unique_temporary(&download_root, &operation_id)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| "download_unavailable".to_string())?;
    let result = (|| {
        update(operation, |state| state.phase = DownloadPhase::Downloading);
        let mut received = 0u64;
        let mut digest = Sha256::new();
        transport.fetch(&spec, &operation.cancelled, &mut |chunk| {
            received = received
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| "download_size_mismatch".to_string())?;
            if received > spec.size || received > MAX_ARTIFACT_BYTES {
                return Err("download_size_mismatch".to_string());
            }
            file.write_all(chunk)
                .map_err(|_| "download_write_failed".to_string())?;
            digest.update(chunk);
            update(operation, |state| state.received_bytes = received);
            Ok(())
        })?;
        file.sync_all()
            .map_err(|_| "download_write_failed".to_string())?;
        drop(file);
        if received != spec.size || format!("{:x}", digest.finalize()) != spec.sha256 {
            return Err("download_hash_mismatch".to_string());
        }
        update(operation, |state| state.phase = DownloadPhase::Verifying);
        let bytes = fs::read(&temporary).map_err(|_| "download_verification_failed".to_string())?;
        if bytes.len() as u64 != spec.size || format!("{:x}", Sha256::digest(&bytes)) != spec.sha256
        {
            return Err("download_verification_failed".to_string());
        }
        let archive_report = match spec.format.as_str() {
            "raw" => {
                if spec.media_type == "text/css"
                    && (std::str::from_utf8(&bytes).is_err() || bytes.contains(&0))
                {
                    return Err("download_content_invalid".to_string());
                }
                None
            }
            "zip" => Some(inspect_zip_bytes(&bytes)?),
            _ => return Err("download_content_invalid".to_string()),
        };
        let receipt =
            content_store::store_remote_fixture_content(app_data_root, &spec.package_id, &bytes)?;
        Ok((receipt.content_identity, archive_report))
    })();
    let _ = fs::remove_file(&temporary);
    result
}

fn terminal(phase: &DownloadPhase) -> bool {
    matches!(
        phase,
        DownloadPhase::Ready | DownloadPhase::Failed | DownloadPhase::Cancelled
    )
}

fn register(operation: Arc<DownloadOperation>) -> Result<(), String> {
    let operations = OPERATIONS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut operations = operations
        .lock()
        .map_err(|_| "download_unavailable".to_string())?;
    if operations.len() >= MAX_OPERATIONS {
        let removable = operations.iter().find_map(|(id, operation)| {
            operation
                .state
                .lock()
                .ok()
                .filter(|state| terminal(&state.phase))
                .map(|_| id.clone())
        });
        if let Some(id) = removable {
            operations.remove(&id);
        } else {
            return Err("download_capacity_reached".to_string());
        }
    }
    let id = operation
        .state
        .lock()
        .map_err(|_| "download_unavailable".to_string())?
        .operation_id
        .clone();
    operations.insert(id, operation);
    Ok(())
}

fn find(operation_id: &str) -> Result<Arc<DownloadOperation>, String> {
    if !operation_id.starts_with("download-")
        || !operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("download_operation_invalid".to_string());
    }
    OPERATIONS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|_| "download_unavailable".to_string())?
        .get(operation_id)
        .cloned()
        .ok_or_else(|| "download_operation_unknown".to_string())
}

pub fn begin(app_data_root: PathBuf, package_id: String) -> Result<ContentDownloadStatus, String> {
    let normalized = package_id.trim();
    let spec = content_store::remote_package_spec(normalized)?;
    let status = ContentDownloadStatus {
        operation_id: operation_id()?,
        package_id: spec.package_id.clone(),
        phase: DownloadPhase::Queued,
        received_bytes: 0,
        expected_bytes: spec.size,
        content_identity: None,
        error_code: None,
        archive_report: None,
    };
    let operation = Arc::new(DownloadOperation {
        state: Mutex::new(status.clone()),
        cancelled: AtomicBool::new(false),
    });
    register(operation.clone())?;
    std::thread::Builder::new()
        .name("betterfy-content-download".to_string())
        .spawn(move || {
            match run_download(
                &app_data_root,
                spec,
                &operation,
                &PinnedHttpsTransport::SAME_ORIGIN,
            ) {
                Ok((identity, archive_report)) => update(&operation, |state| {
                    state.phase = DownloadPhase::Ready;
                    state.content_identity = Some(identity);
                    state.archive_report = archive_report;
                }),
                Err(code) => update(&operation, |state| {
                    state.phase = if code == "download_cancelled" {
                        DownloadPhase::Cancelled
                    } else {
                        DownloadPhase::Failed
                    };
                    state.error_code = Some(code);
                }),
            }
        })
        .map_err(|_| "download_unavailable".to_string())?;
    Ok(status)
}

pub fn status(operation_id: &str) -> Result<ContentDownloadStatus, String> {
    find(operation_id)?
        .state
        .lock()
        .map(|state| state.clone())
        .map_err(|_| "download_unavailable".to_string())
}

pub fn cancel(operation_id: &str) -> Result<ContentDownloadStatus, String> {
    let operation = find(operation_id)?;
    let current = operation
        .state
        .lock()
        .map_err(|_| "download_unavailable".to_string())?
        .clone();
    if !terminal(&current.phase) {
        operation.cancelled.store(true, Ordering::Relaxed);
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureTransport {
        chunks: Vec<Vec<u8>>,
        cancel_after_first: bool,
    }

    impl DownloadTransport for FixtureTransport {
        fn fetch(
            &self,
            _spec: &RemotePackageSpec,
            cancelled: &AtomicBool,
            on_chunk: &mut dyn FnMut(&[u8]) -> Result<(), String>,
        ) -> Result<(), String> {
            for (index, chunk) in self.chunks.iter().enumerate() {
                if cancelled.load(Ordering::Relaxed) {
                    return Err("download_cancelled".to_string());
                }
                on_chunk(chunk)?;
                if index == 0 && self.cancel_after_first {
                    cancelled.store(true, Ordering::Relaxed);
                }
            }
            if cancelled.load(Ordering::Relaxed) {
                return Err("download_cancelled".to_string());
            }
            Ok(())
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "betterfy-download-{name}-{}",
            NEXT_OPERATION.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn operation(spec: &RemotePackageSpec) -> DownloadOperation {
        DownloadOperation {
            state: Mutex::new(ContentDownloadStatus {
                operation_id: operation_id().expect("operation id"),
                package_id: spec.package_id.clone(),
                phase: DownloadPhase::Queued,
                received_bytes: 0,
                expected_bytes: spec.size,
                content_identity: None,
                error_code: None,
                archive_report: None,
            }),
            cancelled: AtomicBool::new(false),
        }
    }

    #[test]
    fn pinned_binary_fetch_never_accepts_short_large_or_substituted_bytes() {
        let mut spec = content_store::remote_package_spec("fixture.ambient-violet").unwrap();
        let payload = b"binary-tree";
        spec.size = payload.len() as u64;
        spec.sha256 = format!("{:x}", Sha256::digest(payload));
        let cancelled = AtomicBool::new(false);
        let good = FixtureTransport {
            chunks: vec![payload[..4].to_vec(), payload[4..].to_vec()],
            cancel_after_first: false,
        };
        assert_eq!(
            fetch_verified_binary_with(&good, &spec, &cancelled).unwrap(),
            payload
        );
        for chunks in [vec![b"short".to_vec()], vec![b"wrong-tree!".to_vec()]] {
            let bad = FixtureTransport {
                chunks,
                cancel_after_first: false,
            };
            assert!(fetch_verified_binary_with(&bad, &spec, &cancelled).is_err());
        }
        let cancelled_midstream = FixtureTransport {
            chunks: vec![payload[..4].to_vec(), payload[4..].to_vec()],
            cancel_after_first: true,
        };
        assert_eq!(
            fetch_verified_binary_with(&cancelled_midstream, &spec, &AtomicBool::new(false))
                .err()
                .as_deref(),
            Some("download_cancelled")
        );
    }

    #[test]
    fn tree_download_rejects_a_substituted_compiled_contract_before_network() {
        let mut forged = crate::tree_pilot::tree_resources()[0].clone();
        forged.bytes += 1;
        assert_eq!(
            fetch_pinned_pilot_resource(
                crate::tree_pilot::PACKAGE_ID,
                &forged,
                &AtomicBool::new(false),
            )
            .err()
            .as_deref(),
            Some("download_contract_invalid")
        );
    }

    #[test]
    fn streams_and_publishes_only_after_hash_verification() {
        let root = temp_root("success");
        let spec = content_store::remote_package_spec("fixture.ambient-violet").expect("spec");
        let payload = include_bytes!("../fixtures/payloads/ambient-violet.css");
        let op = operation(&spec);
        let result = run_download(
            &root,
            spec,
            &op,
            &FixtureTransport {
                chunks: vec![payload[..17].to_vec(), payload[17..].to_vec()],
                cancel_after_first: false,
            },
        )
        .expect("download");
        assert!(result.0.starts_with("sha256:"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn mismatch_and_cancellation_publish_nothing() {
        for (name, chunks, cancel, expected) in [
            (
                "mismatch",
                vec![b"wrong".to_vec()],
                false,
                "download_hash_mismatch",
            ),
            (
                "cancel",
                vec![b"partial".to_vec(), b"rest".to_vec()],
                true,
                "download_cancelled",
            ),
        ] {
            let root = temp_root(name);
            let spec = content_store::remote_package_spec("fixture.ambient-violet").expect("spec");
            let op = operation(&spec);
            assert_eq!(
                run_download(
                    &root,
                    spec,
                    &op,
                    &FixtureTransport {
                        chunks,
                        cancel_after_first: cancel
                    },
                )
                .err()
                .as_deref(),
                Some(expected)
            );
            let objects = root.join("content-v1").join("objects").join("sha256");
            assert!(fs::read_dir(objects).expect("objects").next().is_none());
            fs::remove_dir_all(root).expect("cleanup");
        }
    }

    #[test]
    fn wardrobe_downloads_follow_only_the_cdn_suffix_and_only_there_allow_a_query() {
        let wardrobe = PinnedHttpsTransport::WARDROBE;
        let cdn = Url::parse("https://us.aws.cdn.hf.co/xet-bridge-us/a/b?Expires=1&Signature=x")
            .expect("url");
        assert!(wardrobe.follows(&cdn));
        assert!(validate_url_with(&cdn, true).is_ok());
        assert_eq!(
            validate_url_with(&cdn, false).err().as_deref(),
            Some("download_url_invalid")
        );
        for blocked in [
            "https://cdn.hf.co/x",
            "https://evilcdn.hf.co/x",
            "https://cdn.hf.co.evil.example/x",
            "https://us.aws.cdn.hf.co.evil.example/x",
            "https://huggingface.co/x",
            "https://example.com/x",
        ] {
            let url = Url::parse(blocked).expect("url");
            assert!(!wardrobe.follows(&url), "{blocked}");
        }
        // The default transport never leaves the first URL's origin.
        assert!(!PinnedHttpsTransport::SAME_ORIGIN.follows(&cdn));
        // A fragment, credentials, a plain-HTTP or non-443 target stay invalid
        // even where a query is allowed.
        for invalid in [
            "https://us.aws.cdn.hf.co/x#frag",
            "https://user@us.aws.cdn.hf.co/x",
            "http://us.aws.cdn.hf.co/x",
            "https://us.aws.cdn.hf.co:8443/x",
        ] {
            let url = Url::parse(invalid).expect("url");
            assert!(validate_url_with(&url, true).is_err(), "{invalid}");
        }
    }

    #[test]
    fn wardrobe_archive_fetch_accepts_only_the_pinned_bytes() {
        let manifest = &mirrored_manifest(b"the pinned zip bytes");
        let cancelled = AtomicBool::new(false);
        for chunks in [
            vec![b"short".to_vec()],
            vec![vec![0u8; 1024], vec![1u8; 1024]],
        ] {
            let bad = FixtureTransport {
                chunks,
                cancel_after_first: false,
            };
            assert!(fetch_wardrobe_archive_with(&bad, manifest, &cancelled).is_err());
        }
        let cancelled_midstream = FixtureTransport {
            chunks: vec![vec![0u8; 8], vec![0u8; 8]],
            cancel_after_first: true,
        };
        assert_eq!(
            fetch_wardrobe_archive_with(&cancelled_midstream, manifest, &AtomicBool::new(false))
                .err()
                .as_deref(),
            Some("download_cancelled")
        );
    }

    /// Answers per URL, so a test can make one mirror miss, serve other bytes
    /// or work. Records the URLs it was asked for, in order.
    struct MirrorTransport {
        answers: BTreeMap<String, Result<Vec<u8>, String>>,
        asked: Mutex<Vec<String>>,
    }

    impl DownloadTransport for MirrorTransport {
        fn fetch(
            &self,
            spec: &RemotePackageSpec,
            cancelled: &AtomicBool,
            on_chunk: &mut dyn FnMut(&[u8]) -> Result<(), String>,
        ) -> Result<(), String> {
            self.asked
                .lock()
                .expect("asked")
                .push(spec.download_url.clone());
            match self.answers.get(&spec.download_url) {
                Some(Ok(bytes)) => {
                    for chunk in bytes.chunks(7) {
                        if cancelled.load(Ordering::Relaxed) {
                            return Err("download_cancelled".to_string());
                        }
                        on_chunk(chunk)?;
                    }
                    Ok(())
                }
                Some(Err(code)) => Err(code.clone()),
                None => Err("download_http_failed".to_string()),
            }
        }
    }

    fn mirrored_manifest(payload: &[u8]) -> crate::wardrobe::WardrobeManifest {
        let payload = payload.to_vec();
        crate::wardrobe::synthetic_item(
            "mirror-test",
            &[("models/heroes/keeper_of_the_light/a.vmdl_c", b"a")],
            |value| {
                value["archive"]["zipBytes"] = payload.len().into();
                value["archive"]["zipSha256"] = format!("{:x}", Sha256::digest(&payload)).into();
            },
        )
        .0
    }

    #[test]
    fn a_missing_or_wrong_mirror_falls_through_to_the_next_one() {
        let payload = b"the pinned zip bytes".to_vec();
        let manifest = mirrored_manifest(&payload);
        let [first, second] = [0, 1].map(|index| manifest.source.urls[index].clone());
        let cancelled = AtomicBool::new(false);
        let run = |first_answer: Option<Result<Vec<u8>, String>>,
                   second_answer: Option<Result<Vec<u8>, String>>| {
            let mut answers = BTreeMap::new();
            if let Some(answer) = first_answer {
                answers.insert(first.clone(), answer);
            }
            if let Some(answer) = second_answer {
                answers.insert(second.clone(), answer);
            }
            let transport = MirrorTransport {
                answers,
                asked: Mutex::new(Vec::new()),
            };
            let result = fetch_wardrobe_archive_with(&transport, &manifest, &cancelled);
            let asked = transport.asked.lock().expect("asked").clone();
            (result, asked)
        };

        // The first mirror works: the second is never contacted.
        let (result, asked) = run(Some(Ok(payload.clone())), Some(Ok(b"other".to_vec())));
        assert_eq!(result.expect("first mirror"), payload);
        assert_eq!(asked, vec![first.clone()]);

        // A 404 (the file is on one mirror only), a refused connection and a
        // blocked address each fall through.
        for failure in [
            "download_http_failed",
            "download_transport_failed",
            "download_address_blocked",
        ] {
            let (result, asked) = run(Some(Err(failure.to_string())), Some(Ok(payload.clone())));
            assert_eq!(result.expect("second mirror"), payload, "{failure}");
            assert_eq!(asked, vec![first.clone(), second.clone()], "{failure}");
        }

        // A mirror that serves other bytes (wrong content, short, long) is
        // skipped too, and nothing of it is kept.
        for wrong in [
            b"the pinned zip BYTES".to_vec(),
            b"short".to_vec(),
            [payload.clone(), b"!".to_vec()].concat(),
        ] {
            let (result, asked) = run(Some(Ok(wrong)), Some(Ok(payload.clone())));
            assert_eq!(result.expect("second mirror"), payload);
            assert_eq!(asked, vec![first.clone(), second.clone()]);
        }

        // The download fails only when no mirror yields the pinned bytes, and
        // it reports the last mirror's error.
        let (result, asked) = run(Some(Ok(b"wrong".to_vec())), None);
        assert_eq!(result.err().as_deref(), Some("download_http_failed"));
        assert_eq!(asked, vec![first.clone(), second.clone()]);
        let (result, _) = run(None, Some(Ok(b"wrong".to_vec())));
        assert_eq!(result.err().as_deref(), Some("download_hash_mismatch"));
    }

    /// A raw VPK item has no ZIP: the VPK is the download, pinned by its own
    /// size and SHA-256, and a mirror serving anything else is skipped.
    #[test]
    fn a_raw_vpk_is_fetched_by_its_own_pin_with_the_same_mirror_walk() {
        let (manifest, archive) = crate::wardrobe::synthetic_pack(
            "mega-kill",
            "raw-fetch",
            crate::wardrobe::SourceKind::Vpk,
            &[("sounds/vo/announcer/kill.vsnd_c", b"sound")],
        );
        assert_eq!(manifest.archive.download_bytes(), archive.len() as u64);
        let first = manifest.source.urls[0].clone();
        let second = manifest.source.urls[1].clone();
        assert!(first.ends_with(".vpk") && second.ends_with(".vpk"));
        let transport = MirrorTransport {
            answers: BTreeMap::from([
                (first.clone(), Ok(b"not the pinned vpk".to_vec())),
                (second.clone(), Ok(archive.clone())),
            ]),
            asked: Mutex::new(Vec::new()),
        };
        let bytes = fetch_wardrobe_archive_with(&transport, &manifest, &AtomicBool::new(false))
            .expect("the second mirror has the pinned VPK");
        assert_eq!(bytes, archive);
        assert_eq!(*transport.asked.lock().expect("asked"), vec![first, second]);
    }

    #[test]
    fn cancelling_stops_the_mirror_walk() {
        let payload = b"the pinned zip bytes".to_vec();
        let manifest = mirrored_manifest(&payload);
        let transport = MirrorTransport {
            answers: BTreeMap::from([
                (
                    manifest.source.urls[0].clone(),
                    Err("download_cancelled".to_string()),
                ),
                (manifest.source.urls[1].clone(), Ok(payload)),
            ]),
            asked: Mutex::new(Vec::new()),
        };
        assert_eq!(
            fetch_wardrobe_archive_with(&transport, &manifest, &AtomicBool::new(false))
                .err()
                .as_deref(),
            Some("download_cancelled")
        );
        assert_eq!(transport.asked.lock().expect("asked").len(), 1);
        // Already cancelled before the first failure is looked at.
        let miss = MirrorTransport {
            answers: BTreeMap::new(),
            asked: Mutex::new(Vec::new()),
        };
        assert_eq!(
            fetch_wardrobe_archive_with(&miss, &manifest, &AtomicBool::new(true))
                .err()
                .as_deref(),
            Some("download_cancelled")
        );
        assert_eq!(miss.asked.lock().expect("asked").len(), 1);
    }

    #[test]
    fn every_mirror_is_a_url_the_transport_accepts() {
        // The first hop of each trusted prefix must pass the transport's own
        // URL rules (HTTPS, port 443, no query) so a mirror is never skipped
        // for its shape.
        let manifest = mirrored_manifest(b"x");
        for url in &manifest.source.urls {
            let parsed = Url::parse(url).expect("url");
            assert!(validate_url_with(&parsed, false).is_ok(), "{url}");
        }
    }

    #[test]
    fn blocks_local_network_targets_and_non_https_urls() {
        assert!(!public_ip("127.0.0.1".parse().expect("ip")));
        assert!(!public_ip("10.1.2.3".parse().expect("ip")));
        assert!(public_ip("1.1.1.1".parse().expect("ip")));
        let invalid = Url::parse("http://example.com/file").expect("url");
        assert_eq!(
            validate_url_with(&invalid, false).err().as_deref(),
            Some("download_url_invalid")
        );
    }
}
