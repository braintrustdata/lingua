use std::path::Path;

fn main() {
    // Generate TypeScript types from Rust types
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let ts_dir = Path::new(&out_dir).join("../../../typescript");
    
    // Create typescript directory
    std::fs::create_dir_all(&ts_dir).unwrap();
    
    // This will be called automatically by ts-rs when we run the build
    // The TypeScript files will be generated to typescript/bindings/
    
    println!("cargo:rerun-if-changed=src/");
}