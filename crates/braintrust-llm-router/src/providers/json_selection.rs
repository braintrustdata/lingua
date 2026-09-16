//! Structural selection for generated types without decoding unrelated native fields.

use serde::de::{DeserializeOwned, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::error::Result;

pub(super) enum Segment {
    Key(&'static str),
    Each,
}

pub(super) fn select<T: DeserializeOwned>(
    body: &[u8],
    path: &[Segment],
) -> Result<Vec<(String, T)>> {
    let mut selected = Vec::new();
    let mut deserializer = lingua::serde_json::Deserializer::from_slice(body);
    Selection {
        path,
        pointer: String::new(),
        selected: &mut selected,
    }
    .deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(selected)
}

struct Selection<'a, T> {
    path: &'a [Segment],
    pointer: String,
    selected: &'a mut Vec<(String, T)>,
}

impl<'de, T: DeserializeOwned> DeserializeSeed<'de> for Selection<'_, T> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<(), D::Error> {
        if self.path.is_empty() {
            self.selected
                .push((self.pointer, T::deserialize(deserializer)?));
            Ok(())
        } else {
            deserializer.deserialize_any(self)
        }
    }
}

impl<'de, T: DeserializeOwned> Visitor<'de> for Selection<'_, T> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("JSON containing a selected generated value")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> std::result::Result<(), M::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if matches!(self.path.first(), Some(Segment::Key(expected)) if key == *expected) {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                map.next_value_seed(Selection {
                    path: &self.path[1..],
                    pointer: format!("{}/{escaped}", self.pointer),
                    selected: self.selected,
                })?;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(())
    }

    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> std::result::Result<(), S::Error> {
        if matches!(self.path.first(), Some(Segment::Each)) {
            let mut index = 0;
            while seq
                .next_element_seed(Selection {
                    path: &self.path[1..],
                    pointer: format!("{}/{index}", self.pointer),
                    selected: self.selected,
                })?
                .is_some()
            {
                index += 1;
            }
        } else {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
        }
        Ok(())
    }

    // A scalar has no descendants. In particular, string content needs no media inspection.
    fn visit_str<E: serde::de::Error>(self, _: &str) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<(), E> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lingua::providers::openai::generated::{ContentInputItemContentList, HilariousType};

    #[test]
    fn selects_generated_parts_without_decoding_unrelated_native_fields() {
        let body = br#"{
            "input": [
                {"type":"future_input_item","opaque":9007199254740993},
                {"role":"user","content":"plain text"},
                {"role":"user","content":[{"type":"input_audio","input_audio":{"data":"cmlm","format":"wav"}}]}
            ],
            "extension":{"content":[{"type":"not_a_generated_type"}]}
        }"#;
        let selected = select::<ContentInputItemContentList>(
            body,
            &[
                Segment::Key("input"),
                Segment::Each,
                Segment::Key("content"),
                Segment::Each,
            ],
        )
        .unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].0, "/input/2/content/0");
        assert_eq!(selected[0].1.input_content_type, HilariousType::InputAudio);
    }

    #[test]
    fn selected_parts_are_validated_by_the_generated_type() {
        let body = br#"{"input":[{"content":[{"type":"input_audio","input_audio":{"data":"cmlm","format":"invalid"}}]}]}"#;
        assert!(select::<ContentInputItemContentList>(
            body,
            &[
                Segment::Key("input"),
                Segment::Each,
                Segment::Key("content"),
                Segment::Each,
            ]
        )
        .is_err());
    }
}
