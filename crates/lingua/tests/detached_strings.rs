use lingua::processing::{
    import_and_deduplicate_messages, import_messages_from_spans, DetachedStrings, Span,
};
use std::fs;
use std::path::{Path, PathBuf};

fn import_case_paths() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../payloads/import-cases");
    let mut paths: Vec<_> = fs::read_dir(dir)
        .expect("payloads/import-cases should be readable")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            name.ends_with(".json") && !name.ends_with(".assertions.json")
        })
        .collect();
    paths.sort();
    paths
}

fn spans(value: &serde_json::Value) -> Vec<Span> {
    let text = value.to_string();
    if value.is_array() {
        lingua::serde_json::from_str(&text).expect("span array")
    } else {
        vec![lingua::serde_json::from_str(&text).expect("span object")]
    }
}

fn import(spans: Vec<Span>, dedup: bool) -> serde_json::Value {
    let messages = if dedup {
        import_and_deduplicate_messages(spans)
    } else {
        import_messages_from_spans(spans)
    };
    serde_json::from_str(&lingua::serde_json::to_string(&messages).unwrap()).unwrap()
}

fn padded(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) if text.len() >= 8 && text.contains(' ') => {
            text.push_str(&" padding".repeat(8));
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(padded),
        serde_json::Value::Object(map) => map.values_mut().for_each(padded),
        _ => {}
    }
}

#[test]
fn import_is_transparent_to_detached_strings() {
    let mut cases = 0;
    for path in import_case_paths() {
        let raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let mut long = raw.clone();
        padded(&mut long);
        for fixture in [raw, long] {
            for dedup in [false, true] {
                cases += 1;
                let expected = import(spans(&fixture), dedup);

                let mut detached_fixture = fixture.clone();
                let mut detached = DetachedStrings::default();
                detached.detach(&mut detached_fixture);
                let mut actual = import(spans(&detached_fixture), dedup);
                let mut cloned = actual.clone();
                detached.attach_cloned(&mut cloned).unwrap();
                detached.attach(&mut actual).unwrap();
                assert_eq!(actual, expected, "{} dedup={dedup}", path.display());
                assert_eq!(
                    cloned,
                    expected,
                    "{} dedup={dedup} (cloned)",
                    path.display()
                );

                if let serde_json::Value::Array(items) = &fixture {
                    let split = items.len() / 2;
                    let mut left = serde_json::Value::Array(items[..split].to_vec());
                    let mut right = serde_json::Value::Array(items[split..].to_vec());
                    let mut left_detached = DetachedStrings::default();
                    let mut right_detached = DetachedStrings::default();
                    left_detached.detach(&mut left);
                    right_detached.detach(&mut right);
                    left_detached.merge(right_detached, [&mut right]).unwrap();
                    let (serde_json::Value::Array(mut merged), serde_json::Value::Array(right)) =
                        (left, right)
                    else {
                        unreachable!();
                    };
                    merged.extend(right);
                    let mut actual = import(spans(&serde_json::Value::Array(merged)), dedup);
                    left_detached.attach(&mut actual).unwrap();
                    assert_eq!(
                        actual,
                        expected,
                        "{} dedup={dedup} (merged)",
                        path.display()
                    );
                }
            }
        }
    }
    assert!(cases > 100);
}
