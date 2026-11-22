use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // apps/server/package.json version — the same `serverBuild` the original
    // WS negotiate layer reports (read at compile time; Cargo re-runs when the
    // file changes).
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let pkg = PathBuf::from(manifest).join("../../apps/server/package.json");
    println!("cargo:rerun-if-changed={}", pkg.display());
    let version = fs::read_to_string(&pkg)
        .ok()
        .and_then(|text| {
            let mut it = text.split("\"version\"");
            it.next()?;
            let rest = it.next()?;
            let start = rest.find('"')? + 1;
            let end = rest[start..].find('"')? + start;
            Some(rest[start..end].to_string())
        })
        .unwrap_or_else(|| "0.0.0".to_string());
    println!("cargo:rustc-env=SYNARA_SERVER_BUILD={version}");
}
