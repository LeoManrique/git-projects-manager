fn main() {
    println!("cargo:rerun-if-env-changed=GOOGLE_OAUTH_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=SYNC_SERVER_URL");
    tauri_build::build()
}
