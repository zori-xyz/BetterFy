fn main() {
    // The sequence of the catalog committed with this build is the lowest one
    // the app will ever accept, so a replayed older (still validly signed)
    // catalog cannot roll a fresh install back.
    let catalog = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../website/public/bot/catalog/index.json");
    println!("cargo:rerun-if-changed={}", catalog.display());
    let text = std::fs::read_to_string(&catalog).expect("website/public/bot/catalog/index.json");
    let sequence = text
        .split("\"sequence\":")
        .nth(1)
        .and_then(|rest| {
            rest.trim_start()
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|digits| digits.parse::<u64>().ok())
        })
        .expect("catalog index must contain a numeric sequence");
    println!("cargo:rustc-env=BETTERFY_MIN_CATALOG_SEQUENCE={sequence}");
    tauri_build::build()
}
