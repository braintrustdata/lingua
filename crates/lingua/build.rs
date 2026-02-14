use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn main() {
    let workspace = workspace_root();

    // Create TypeScript bindings directory structure
    let bindings_dir = workspace.join("bindings/typescript/src/generated");
    fs::create_dir_all(&bindings_dir).unwrap();

    // ts-rs will automatically export types marked with #[ts(export)]
    // to the directory specified in TS_RS_EXPORT_DIR
    println!(
        "cargo:rustc-env=TS_RS_EXPORT_DIR={}",
        bindings_dir.display()
    );

    // Only rerun if source files change
    println!("cargo:rerun-if-changed=src/universal/");

    // Generate test cases from payloads directory
    generate_test_cases(&workspace);
    generate_chat_completions_test_cases(&workspace);
    generate_anthropic_test_cases(&workspace);
    generate_google_test_cases(&workspace);
}

fn generate_test_cases(workspace: &Path) {
    let snapshots_dir = workspace.join("payloads/snapshots");

    // Tell cargo to re-run if the snapshots directory changes
    println!("cargo:rerun-if-changed={}", snapshots_dir.display());

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_tests.rs");

    if !snapshots_dir.exists() {
        // Create empty generated tests file if no snapshots directory
        fs::write(&dest_path, "// No test cases found").unwrap();
        return;
    }

    let mut generated_tests = String::new();

    generated_tests.push_str("// Auto-generated test cases from payloads/snapshots directory\n");
    generated_tests.push_str("// DO NOT EDIT - regenerated on each build\n\n");

    // Scan for test case directories
    if let Ok(entries) = fs::read_dir(&snapshots_dir) {
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

            // Check if this test case has responses directory
            let responses_dir = path.join("responses");
            if !responses_dir.exists() {
                continue;
            }

            // Generate tests for both turns if they exist
            if responses_dir.join("request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_first_turn", test_case_name);
                let full_case_name = format!("{}_responses_first_turn", test_case_name);

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

            if responses_dir.join("followup-request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_followup_turn", test_case_name);
                let full_case_name = format!("{}_responses_followup_turn", test_case_name);

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

fn generate_chat_completions_test_cases(workspace: &Path) {
    let snapshots_dir = workspace.join("payloads/snapshots");

    // Tell cargo to re-run if the snapshots directory changes
    println!("cargo:rerun-if-changed={}", snapshots_dir.display());

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_chat_completions_tests.rs");

    if !snapshots_dir.exists() {
        // Create empty generated tests file if no snapshots directory
        fs::write(&dest_path, "// No test cases found").unwrap();
        return;
    }

    let mut generated_tests = String::new();

    generated_tests.push_str(
        "// Auto-generated chat completion test cases from payloads/snapshots directory\n",
    );
    generated_tests.push_str("// DO NOT EDIT - regenerated on each build\n\n");

    // Scan for test case directories
    if let Ok(entries) = fs::read_dir(&snapshots_dir) {
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

            // Check if this test case has chat-completions directory
            let chat_completions_dir = path.join("chat-completions");
            if !chat_completions_dir.exists() {
                continue;
            }

            // Generate tests for both turns if they exist
            if chat_completions_dir.join("request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_first_turn", test_case_name);
                let full_case_name = format!("{}_chat-completions_first_turn", test_case_name);

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

            if chat_completions_dir.join("followup-request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_followup_turn", test_case_name);
                let full_case_name = format!("{}_chat-completions_followup_turn", test_case_name);

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

fn generate_anthropic_test_cases(workspace: &Path) {
    let snapshots_dir = workspace.join("payloads/snapshots");

    // Tell cargo to re-run if the snapshots directory changes
    println!("cargo:rerun-if-changed={}", snapshots_dir.display());

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_anthropic_tests.rs");

    if !snapshots_dir.exists() {
        // Create empty generated tests file if no snapshots directory
        fs::write(&dest_path, "// No test cases found").unwrap();
        return;
    }

    let mut generated_tests = String::new();

    generated_tests
        .push_str("// Auto-generated Anthropic test cases from payloads/snapshots directory\n");
    generated_tests.push_str("// DO NOT EDIT - regenerated on each build\n\n");

    // Scan for test case directories
    if let Ok(entries) = fs::read_dir(&snapshots_dir) {
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

            // Check if this test case has anthropic directory
            let anthropic_dir = path.join("anthropic");
            if !anthropic_dir.exists() {
                continue;
            }

            // Generate tests for both turns if they exist
            if anthropic_dir.join("request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_first_turn", test_case_name);
                let full_case_name = format!("{}_anthropic_first_turn", test_case_name);

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

            if anthropic_dir.join("followup-request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_followup_turn", test_case_name);
                let full_case_name = format!("{}_anthropic_followup_turn", test_case_name);

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

fn generate_google_test_cases(workspace: &Path) {
    let snapshots_dir = workspace.join("payloads/snapshots");

    // Tell cargo to re-run if the snapshots directory changes
    println!("cargo:rerun-if-changed={}", snapshots_dir.display());

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_google_tests.rs");

    if !snapshots_dir.exists() {
        // Create empty generated tests file if no snapshots directory
        fs::write(&dest_path, "// No test cases found").unwrap();
        return;
    }

    let mut generated_tests = String::new();

    generated_tests
        .push_str("// Auto-generated Google test cases from payloads/snapshots directory\n");
    generated_tests.push_str("// DO NOT EDIT - regenerated on each build\n\n");

    // Scan for test case directories
    if let Ok(entries) = fs::read_dir(&snapshots_dir) {
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

            // Check if this test case has google directory
            let google_dir = path.join("google");
            if !google_dir.exists() {
                continue;
            }

            // Generate tests for both turns if they exist
            if google_dir.join("request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_first_turn", test_case_name);
                let full_case_name = format!("{}_google_first_turn", test_case_name);

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

            if google_dir.join("followup-request.json").exists() {
                let test_fn_name = format!("test_roundtrip_{}_followup_turn", test_case_name);
                let full_case_name = format!("{}_google_followup_turn", test_case_name);

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
