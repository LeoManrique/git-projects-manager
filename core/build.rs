use std::fs;
use std::path::PathBuf;

fn main() {
    // Tell Cargo to rerun this build script if these env vars change
    println!("cargo:rerun-if-env-changed=GOOGLE_OAUTH_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=GOOGLE_OAUTH_CLIENT_SECRET");
    println!("cargo:rerun-if-env-changed=SYNC_SERVER_URL");

    // Target server/.env directly
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_dir = manifest_dir.parent().unwrap();
    let env_path = workspace_dir.join("server").join(".env");

    if env_path.exists() && env_path.is_file() {
        println!("cargo:rerun-if-changed={}", env_path.display());
        if let Ok(content) = fs::read_to_string(&env_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, val)) = line.split_once('=') {
                    let key = key.trim();
                    let val = val.trim();
                    // Strip quotes if present
                    let val = val.trim_matches(|c| c == '\'' || c == '"');

                    // Set the env var for compiling core if not already set in ambient env
                    if std::env::var(key).is_err() {
                        println!("cargo:rustc-env={key}={val}");
                    }
                }
            }
        }
    }
}
