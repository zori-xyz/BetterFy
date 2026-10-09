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
/// only decides where BetterFy is willing to connect.
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

/// Downloads a wardrobe item's pinned ZIP. The URL and the expected size and
/// SHA-256 come from the embedded allowlist entry only.
pub(crate) fn fetch_pinned_wardrobe_archive(
    manifest: &crate::wardrobe::WardrobeManifest,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, String> {
    fetch_wardrobe_archive_with(&PinnedHttpsTransport::WARDROBE, manifest, cancelled)
}

fn fetch_wardrobe_archive_with<T: DownloadTransport>(
    transport: &T,
    manifest: &crate::wardrobe::WardrobeManifest,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, String> {
    let spec = RemotePackageSpec {
        package_id: manifest.id.clone(),
        version: "pinned-archive".to_string(),
        format: "zip".to_string(),
        file_name: "wardrobe-archive.zip".to_string(),
        download_url: manifest.source.url.clone(),
        media_type: "application/zip".to_string(),
        size: manifest.archive.zip_bytes,
        sha256: manifest.archive.zip_sha256.clone(),
    };
    fetch_verified_binary_with(transport, &spec, cancelled)
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
        let manifest = crate::wardrobe::find("heroes-scarlet-keeper-of-the-light")
            .expect("registry")
            .expect("scarlet keeper");
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
