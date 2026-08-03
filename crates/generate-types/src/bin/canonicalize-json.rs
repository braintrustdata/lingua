use big_serde_json as serde_json;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[derive(Debug)]
struct AtomicReplaceError {
    temp_path: PathBuf,
    operation_error: io::Error,
    cleanup_error: io::Error,
}

impl fmt::Display for AtomicReplaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to update JSON file: {}; failed to remove temporary file {}: {}",
            self.operation_error,
            self.temp_path.display(),
            self.cleanup_error
        )
    }
}

impl Error for AtomicReplaceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.operation_error)
    }
}

fn canonicalize_json(source: &str) -> Result<String, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(source)?;
    Ok(format!("{}\n", serde_json::to_string_pretty(&value)?))
}

fn combine_cleanup_result(
    temp_path: PathBuf,
    operation_error: io::Error,
    cleanup_result: io::Result<()>,
) -> Result<io::Error, AtomicReplaceError> {
    match cleanup_result {
        Ok(()) => Ok(operation_error),
        Err(cleanup_error) if cleanup_error.kind() == io::ErrorKind::NotFound => {
            Ok(operation_error)
        }
        Err(cleanup_error) => Err(AtomicReplaceError {
            temp_path,
            operation_error,
            cleanup_error,
        }),
    }
}

fn fail_after_cleanup(
    temp_file: NamedTempFile,
    operation_error: io::Error,
) -> Result<bool, Box<dyn Error>> {
    let temp_path = temp_file.path().to_path_buf();
    let cleanup_result = temp_file.close();
    match combine_cleanup_result(temp_path, operation_error, cleanup_result) {
        Ok(operation_error) => Err(Box::new(operation_error)),
        Err(combined_error) => Err(Box::new(combined_error)),
    }
}

fn canonicalize_json_file(file_path: &Path) -> Result<bool, Box<dyn Error>> {
    let source = fs::read_to_string(file_path)?;
    let canonical = canonicalize_json(&source)?;

    if source == canonical {
        return Ok(false);
    }

    let metadata = fs::metadata(file_path)?;
    let directory = file_path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "file has no parent directory")
    })?;
    let mut temp_file = tempfile::Builder::new()
        .prefix(".canonical-json-")
        .suffix(".tmp")
        .tempfile_in(directory)?;

    if let Err(error) = temp_file.write_all(canonical.as_bytes()) {
        return fail_after_cleanup(temp_file, error);
    }
    if let Err(error) = temp_file.flush() {
        return fail_after_cleanup(temp_file, error);
    }
    if let Err(error) = temp_file.as_file().set_permissions(metadata.permissions()) {
        return fail_after_cleanup(temp_file, error);
    }

    match temp_file.persist(file_path) {
        Ok(_) => Ok(true),
        Err(error) => fail_after_cleanup(error.file, error.error),
    }
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let Some(file_path) = arguments.next() else {
        eprintln!("Usage: canonicalize-json <json-file>");
        std::process::exit(1);
    };
    if arguments.next().is_some() {
        eprintln!("Usage: canonicalize-json <json-file>");
        std::process::exit(1);
    }

    let file_path = PathBuf::from(file_path);
    match canonicalize_json_file(&file_path) {
        Ok(changed) => println!(
            "{}",
            if changed {
                format!("Canonicalized {}", file_path.display())
            } else {
                format!("{} is canonical", file_path.display())
            }
        ),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_keys_recursively_and_preserves_array_order() {
        let source =
            r#"{"z":{"second":2,"first":1},"array":[{"z":1,"a":2},"unchanged",3],"a":true}"#;
        let expected = r#"{
  "a": true,
  "array": [
    {
      "a": 2,
      "z": 1
    },
    "unchanged",
    3
  ],
  "z": {
    "first": 1,
    "second": 2
  }
}
"#;

        assert_eq!(canonicalize_json(source).unwrap(), expected);
    }

    #[test]
    fn preserves_arbitrary_precision_numbers() {
        let source =
            r#"{"z":9007199254740993,"exponent":1e+400,"decimal":0.12345678901234567890123456789}"#;
        let canonical = canonicalize_json(source).unwrap();

        assert!(canonical.contains("0.12345678901234567890123456789"));
        assert!(canonical.contains("1e+400"));
        assert!(canonical.contains("9007199254740993"));
    }

    #[test]
    fn canonicalizes_files_atomically_and_idempotently() {
        let directory = tempfile::tempdir().unwrap();
        let file_path = directory.path().join("discovery.json");
        fs::write(&file_path, r#"{"z":1,"a":{"z":2,"a":3}}"#).unwrap();

        assert!(canonicalize_json_file(&file_path).unwrap());
        let canonical = fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            canonical,
            "{\n  \"a\": {\n    \"a\": 3,\n    \"z\": 2\n  },\n  \"z\": 1\n}\n"
        );
        assert!(!canonicalize_json_file(&file_path).unwrap());
    }

    #[test]
    fn leaves_the_original_file_untouched_when_parsing_fails() {
        let directory = tempfile::tempdir().unwrap();
        let file_path = directory.path().join("discovery.json");
        let invalid_json = r#"{"not":"complete""#;
        fs::write(&file_path, invalid_json).unwrap();

        assert!(canonicalize_json_file(&file_path).is_err());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), invalid_json);
    }

    #[test]
    fn ignores_an_expected_missing_temporary_file_during_cleanup() {
        let operation_error = io::Error::other("write failed");
        let cleanup_error = io::Error::new(io::ErrorKind::NotFound, "file not found");

        let returned_error = combine_cleanup_result(
            PathBuf::from("temporary.json"),
            operation_error,
            Err(cleanup_error),
        )
        .unwrap();

        assert_eq!(returned_error.to_string(), "write failed");
    }

    #[test]
    fn combines_the_operation_and_cleanup_errors() {
        let operation_error = io::Error::other("write failed");
        let cleanup_error = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");

        let combined_error = combine_cleanup_result(
            PathBuf::from("temporary.json"),
            operation_error,
            Err(cleanup_error),
        )
        .unwrap_err();

        assert!(combined_error.to_string().contains("write failed"));
        assert!(combined_error.to_string().contains("permission denied"));
    }

    #[test]
    fn provider_pipeline_uses_the_rust_canonicalizer_for_google() {
        let pipeline = include_str!("../../../../pipelines/generate-provider-types.sh");
        assert!(pipeline.contains(
            "cargo run --quiet --manifest-path \"$PROJECT_ROOT/Cargo.toml\" --bin canonicalize-json -- \"$SPEC_FILE\""
        ));
    }

    #[test]
    fn checked_in_google_discovery_spec_is_canonical() {
        let spec_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/google/discovery.json");
        let source = fs::read_to_string(spec_path).unwrap();

        assert_eq!(canonicalize_json(&source).unwrap(), source);
    }
}
