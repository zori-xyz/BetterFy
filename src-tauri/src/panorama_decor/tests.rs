use super::*;
use crate::panorama_layout::{self, Node};
use crate::vpk;
use std::collections::BTreeMap;
use std::io::Read;

// ---------- the local audit of the cached catalog archives ----------

fn read_files(path: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let bytes = std::fs::read(path).expect("archive");
    let vpk_bytes = if path.extension().is_some_and(|e| e == "zip") {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip");
        let mut found = None;
        for index in 0..zip.len() {
            let mut file = zip.by_index(index).expect("entry");
            if file.name().to_ascii_lowercase().ends_with("_dir.vpk") {
                let mut data = Vec::new();
                file.read_to_end(&mut data).expect("read");
                found = Some(data);
                break;
            }
        }
        found.expect("vpk in zip")
    } else {
        bytes
    };
    vpk::read_embedded_directory(&vpk_bytes)
        .expect("vpk")
        .into_iter()
        .map(|entry| {
            let payload = entry.payload().into_owned();
            (entry.path.to_ascii_lowercase(), payload)
        })
        .collect()
}

fn valve_paths() -> BTreeSet<String> {
    let path = std::env::var("VALVE_LIST").expect("VALVE_LIST");
    std::fs::read_to_string(path)
        .expect("list")
        .lines()
        .filter_map(|line| line.split(' ').next())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn handlers_of(node: &Node, out: &mut BTreeSet<(String, String, String, String)>) {
    if node.kind() == "PANEL" {
        let id = node
            .children()
            .iter()
            .find(|c| c.kind() == "PANEL_ATTRIBUTE" && c.name() == Some("id"))
            .and_then(|c| c.children().first())
            .and_then(Node::name)
            .unwrap_or("")
            .to_string();
        for child in node.children() {
            if child.kind() == "PANEL_ATTRIBUTE" {
                let name = child.name().unwrap_or("");
                if name.len() >= 2 && name[..2].eq_ignore_ascii_case("on") {
                    let code = child
                        .children()
                        .first()
                        .and_then(Node::name)
                        .unwrap_or("")
                        .to_string();
                    out.insert((
                        node.name().unwrap_or("").to_string(),
                        id.clone(),
                        name.to_string(),
                        code,
                    ));
                }
            }
        }
    }
    for child in node.children() {
        handlers_of(child, out);
    }
}

/// Reads the cached archives of the Panorama categories (`DECOR_CACHE`, one
/// folder per catalog category) and prints what the proofs make of them.
/// Local only: it needs the downloaded archives and Valve's file list
/// (`VALVE_LIST`). With `DECOR_PRINT_PINS=1` it also prints every event
/// binding found, as source lines for `pins.rs` (to be reviewed by hand).
#[test]
#[ignore]
fn audit_cached_decor_archives() {
    let root = std::path::PathBuf::from(std::env::var("DECOR_CACHE").expect("DECOR_CACHE"));
    let valve = valve_paths();
    let allow = std::env::var("DECOR_NO_BINDINGS").is_err();
    let mut pins = BTreeSet::new();
    for (folder, category) in [
        ("backgrounds", Category::Backgrounds),
        ("huds", Category::Huds),
        ("versus-screens", Category::VersusScreens),
    ] {
        let mut archives = 0usize;
        let mut fully_accepted = 0usize;
        let mut tiers: BTreeMap<(Kind, Tier), usize> = BTreeMap::new();
        let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
        let mut details: BTreeMap<String, usize> = BTreeMap::new();
        let mut files = 0usize;
        let mut per_archive: Vec<String> = Vec::new();
        let mut names = std::fs::read_dir(root.join(folder))
            .expect("folder")
            .map(|e| e.expect("entry").path())
            .filter(|p| p.extension().is_none_or(|e| e != "part"))
            .collect::<Vec<_>>();
        names.sort();
        for path in names {
            archives += 1;
            let entries = read_files(&path);
            for (name, bytes) in &entries {
                if name.ends_with(".vxml_c") {
                    if let Ok(decoded) = panorama_layout::decode(bytes) {
                        handlers_of(&decoded.root, &mut pins);
                    }
                }
            }
            let borrowed = entries
                .iter()
                .map(|(p, b)| (p.clone(), b.as_slice()))
                .collect::<Vec<_>>();
            let verdicts = prove_archive(category, &borrowed, &valve, allow);
            let mut ok = true;
            for (file, verdict) in &verdicts {
                files += 1;
                match verdict {
                    Ok(proof) => *tiers.entry((proof.kind, proof.tier)).or_default() += 1,
                    Err(refusal) => {
                        ok = false;
                        *refusals.entry(refusal.code().to_string()).or_default() += 1;
                        *details.entry(refusal.detail()).or_default() += 1;
                        per_archive.push(format!(
                            "{}: {file}: {}",
                            path.file_name().unwrap().to_string_lossy(),
                            refusal.detail()
                        ));
                    }
                }
            }
            if ok {
                fully_accepted += 1;
            }
        }
        println!("== {folder}: {archives} archives, {files} layout/style/video files, {fully_accepted} archives with every such file accepted");
        println!("   accepted by kind and tier: {tiers:?}");
        println!("   refused by code: {refusals:?}");
        let mut ranked = details.into_iter().collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (detail, count) in ranked.iter().take(25) {
            println!("     {count:>4}  {detail}");
        }
        for line in per_archive.iter().take(12) {
            println!("   - {line}");
        }
    }
    // What the earlier rule (image wrappers only) made of the same layouts,
    // and what the other file types in these archives are.
    let (mut old_pass, mut old_scripted, mut old_unproven, mut layouts) = (0, 0, 0, 0);
    let mut other_types: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    let mut rng = 0x2545_F491_4F6C_DD1Du64;
    let mut mutations = 0usize;
    for folder in [
        "backgrounds",
        "huds",
        "versus-screens",
        "item-effects",
        "herofx",
    ] {
        let Ok(dir) = std::fs::read_dir(root.join(folder)) else {
            continue;
        };
        for path in dir.map(|e| e.expect("entry").path()) {
            if path.extension().is_some_and(|e| e == "part") {
                continue;
            }
            for (name, bytes) in read_files(&path) {
                let extension = name.rsplit('.').next().unwrap_or("").to_string();
                if name.ends_with(".vxml_c") {
                    layouts += 1;
                    match panorama_layout::image_wrapper_dependency(&name, &bytes) {
                        Ok(_) => old_pass += 1,
                        Err(panorama_layout::WrapperError::Scripted) => old_scripted += 1,
                        Err(panorama_layout::WrapperError::Unproven) => old_unproven += 1,
                    }
                }
                if kind_of(&name).is_none() {
                    let entry = other_types
                        .entry(format!("{folder}: .{extension}"))
                        .or_default();
                    entry.0 += 1;
                    entry.1 += bytes.len() as u64;
                }
                // Mutate the real layouts, styles and the head of the videos:
                // no panic, whatever the verdict.
                if kind_of(&name).is_some() && bytes.len() < 300_000 {
                    let archive = BTreeSet::new();
                    let ctx = Context {
                        valve: &valve,
                        archive: &archive,
                        allow_reviewed_valve_bindings: true,
                    };
                    for _ in 0..200 {
                        let mut mutated = bytes.clone();
                        for _ in 0..3 {
                            rng ^= rng << 13;
                            rng ^= rng >> 7;
                            rng ^= rng << 17;
                            let at = (rng as usize) % mutated.len().max(1);
                            if let Some(byte) = mutated.get_mut(at) {
                                *byte = (rng >> 24) as u8;
                            }
                        }
                        let _ = match kind_of(&name) {
                            Some(Kind::Layout) => prove_static_layout(&name, &mutated, &ctx),
                            Some(Kind::Style) => prove_style_sheet(&name, &mutated, &ctx),
                            _ => check_webm(&mutated).map(|_| Proof {
                                kind: Kind::Video,
                                tier: Tier::PureDecor,
                                dependencies: BTreeSet::new(),
                                facts: Facts::default(),
                            }),
                        };
                        mutations += 1;
                    }
                }
            }
        }
    }
    println!(
        "== earlier image-wrapper rule over the same {layouts} layouts: {old_pass} accepted, {old_scripted} scripted, {old_unproven} unproven"
    );
    println!("== files of other types in these archives: {other_types:?}");
    println!("== {mutations} mutated real files read without a panic");
    if std::env::var("DECOR_PRINT_PINS").is_ok() {
        for (tag, id, attribute, code) in &pins {
            println!("    ({tag:?}, {id:?}, {attribute:?}, {code:?}),");
        }
    }
    println!("distinct event bindings found: {}", pins.len());
}
