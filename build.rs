use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Create TypeScript bindings directory
    std::fs::create_dir_all("bindings/typescript").unwrap();

    // ts-rs will automatically export types marked with #[ts(export)]
    // to the directory specified in TS_RS_EXPORT_DIR
    println!("cargo:rustc-env=TS_RS_EXPORT_DIR=./bindings/typescript");

    // Only rerun if source files change
    println!("cargo:rerun-if-changed=src/universal/");

    // Generate test cases from payloads directory
    generate_test_cases();
}

fn generate_test_cases() {
    // Tell cargo to re-run if the snapshots directory changes
    println!("cargo:rerun-if-changed=payloads/snapshots");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_tests.rs");

    // Scan the payloads/snapshots directory
    let snapshots_dir = Path::new("payloads/snapshots");

    if !snapshots_dir.exists() {
        // Create empty generated tests file if no snapshots directory
        fs::write(&dest_path, "// No test cases found").unwrap();
        return;
    }

    let mut generated_tests = String::new();

    generated_tests.push_str("// Auto-generated test cases from payloads/snapshots directory\n");
    generated_tests.push_str("// DO NOT EDIT - regenerated on each build\n\n");

    // Scan for test case directories
    if let Ok(entries) = fs::read_dir(snapshots_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let test_case_name = path.file_name().unwrap().to_str().unwrap();

            // Skip hidden directories and cache files
            if test_case_name.starts_with('.') {
                continue;
            }

            // Check if this test case has openai-responses directory
            let openai_responses_dir = path.join("openai-responses");
            if !openai_responses_dir.exists() {
                continue;
            }

            // Generate tests for both turns if they exist
            if openai_responses_dir.join("request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_first_turn", test_case_name);
                let full_case_name = format!("{}_openai-responses_first_turn", test_case_name);

                generated_tests.push_str(&format!(
                    r#"
#[test]
fn {test_fn_name}() {{
    super::run_single_test_case("{full_case_name}")
        .unwrap_or_else(|e| panic!("Test failed for {full_case_name}: {{}}", e));
}}
"#
                ));
            }

            if openai_responses_dir.join("followup-request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_followup_turn", test_case_name);
                let full_case_name = format!("{}_openai-responses_followup_turn", test_case_name);

                generated_tests.push_str(&format!(
                    r#"
#[test]
fn {test_fn_name}() {{
    super::run_single_test_case("{full_case_name}")
        .unwrap_or_else(|e| panic!("Test failed for {full_case_name}: {{}}", e));
}}
"#
                ));
            }
        }
    }

    // Write the generated tests
    fs::write(&dest_path, generated_tests).unwrap();
}
