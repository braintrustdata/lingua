use std::path::Path;

fn main() {
    // Generate TypeScript types from Rust types using ts-rs
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let ts_dir = Path::new(&out_dir).join("../../../typescript");

    // Create typescript directory
    std::fs::create_dir_all(&ts_dir).unwrap();

    // Note: Provider type generation has been moved to scripts/generate-types.rs
    // Run `cargo run --bin generate-types -- all` to generate provider types

    // Only rerun if source files change
    println!("cargo:rerun-if-changed=src/");
}
