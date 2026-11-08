fn main() {
    println!("cargo:rerun-if-env-changed=CHIIKAWA_EMBEDDED_MCP_KEY");
    tauri_build::build();
}
