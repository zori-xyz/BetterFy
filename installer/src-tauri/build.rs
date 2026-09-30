fn main() {
    tauri_build::build();

    // This installer's own Cargo.toml version is its own build number, not
    // the version of the app it embeds. Reading the root package.json at
    // build time keeps the registry's DisplayVersion (and the "already
    // installed" UI text) truthful without relying on someone remembering to
    // bump two version fields in lockstep — a mismatch here previously left
    // every install reporting version "0.1.0" regardless of the real build.
    let package_json = std::fs::read_to_string("../../package.json")
        .expect("failed to read ../../package.json for the embedded app version");
    let parsed: serde_json::Value =
        serde_json::from_str(&package_json).expect("../../package.json is not valid JSON");
    let version = parsed["version"]
        .as_str()
        .expect("../../package.json has no string \"version\" field");
    println!("cargo:rustc-env=BETTERFY_APP_VERSION={version}");
    println!("cargo:rerun-if-changed=../../package.json");
}
