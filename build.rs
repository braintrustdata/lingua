fn main() {
    // Create TypeScript bindings directory
    std::fs::create_dir_all("bindings/typescript").unwrap();

    // ts-rs will automatically export types marked with #[ts(export)]
    // to the directory specified in TS_RS_EXPORT_DIR or bindings/ by default
    std::env::set_var("TS_RS_EXPORT_DIR", "./bindings/typescript");

    // Only rerun if source files change
    println!("cargo:rerun-if-changed=src/universal/");
}
