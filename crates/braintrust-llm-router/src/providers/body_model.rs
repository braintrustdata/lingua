use bytes::Bytes;
use lingua::serde_json::{self, Value};
use lingua::ProviderFormat;

fn body_model_field(format: ProviderFormat) -> Option<&'static str> {
    match format {
        ProviderFormat::ChatCompletions
        | ProviderFormat::Responses
        | ProviderFormat::Anthropic
        | ProviderFormat::Mistral => Some("model"),
        ProviderFormat::Google
        | ProviderFormat::Converse
        | ProviderFormat::BedrockAnthropic
        | ProviderFormat::VertexAnthropic
        | ProviderFormat::Unknown => None,
    }
}

enum BodyModelRewrite {
    Required,
    NotRequired,
    Unknown,
}

#[derive(serde::Deserialize)]
struct BodyModel {
    model: Option<String>,
}

fn body_model_rewrite_status(
    payload: &[u8],
    format: ProviderFormat,
    model: &str,
) -> BodyModelRewrite {
    match body_model_field(format) {
        Some("model") => match serde_json::from_slice::<BodyModel>(payload) {
            Ok(parsed) => {
                if parsed.model.as_deref() == Some(model) {
                    BodyModelRewrite::NotRequired
                } else {
                    BodyModelRewrite::Required
                }
            }
            Err(_) => BodyModelRewrite::Unknown,
        },
        Some(_) | None => BodyModelRewrite::Unknown,
    }
}

pub(crate) fn rewrite_body_model_if_required(
    payload: Bytes,
    format: ProviderFormat,
    model: &str,
) -> Bytes {
    match body_model_rewrite_status(&payload, format, model) {
        BodyModelRewrite::Required => {}
        BodyModelRewrite::NotRequired | BodyModelRewrite::Unknown => return payload,
    }

    let Some(model_field) = body_model_field(format) else {
        return payload;
    };
    let Ok(mut value) = serde_json::from_slice::<Value>(&payload) else {
        return payload;
    };
    let Some(object) = value.as_object_mut() else {
        return payload;
    };
    if object.get(model_field).and_then(Value::as_str) == Some(model) {
        return payload;
    }
    object.insert(model_field.to_string(), Value::String(model.to_string()));
    match serde_json::to_vec(&value) {
        Ok(serialized) => Bytes::from(serialized),
        Err(_) => payload,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_body_model_if_required_leaves_matching_model_bytes_unchanged() {
        let payload = Bytes::from_static(br#"{"model":"gpt-4o","messages":[]}"#);
        let original_ptr = payload.as_ptr();

        let updated =
            rewrite_body_model_if_required(payload, ProviderFormat::ChatCompletions, "gpt-4o");

        assert_eq!(updated.as_ptr(), original_ptr);
    }

    #[test]
    fn rewrite_body_model_if_required_rewrites_mismatched_model_field() {
        let payload = Bytes::from_static(br#"{"model":"gpt-4","messages":[]}"#);

        let updated =
            rewrite_body_model_if_required(payload, ProviderFormat::ChatCompletions, "gpt-4o");
        let value: Value = serde_json::from_slice(&updated).unwrap();

        assert_eq!(value.get("model").and_then(Value::as_str), Some("gpt-4o"));
    }

    #[test]
    fn rewrite_body_model_if_required_leaves_converse_payload_unchanged() {
        let payload = Bytes::from_static(
            br#"{"modelId":"model-a","messages":[{"role":"user","content":[]}]}"#,
        );
        let original_ptr = payload.as_ptr();

        let updated = rewrite_body_model_if_required(payload, ProviderFormat::Converse, "model-b");

        assert_eq!(updated.as_ptr(), original_ptr);
    }

    #[test]
    fn rewrite_body_model_if_required_leaves_google_payload_unchanged() {
        let payload = Bytes::from_static(
            br#"{"model":"gemini-2.5-flash","contents":[{"role":"user","parts":[{"text":"hi"}]}]}"#,
        );
        let original_ptr = payload.as_ptr();

        let updated = rewrite_body_model_if_required(
            payload,
            ProviderFormat::Google,
            "models/gemini-2.5-pro",
        );

        assert_eq!(updated.as_ptr(), original_ptr);
    }

    #[test]
    fn rewrite_body_model_if_required_leaves_unknown_payload_unchanged() {
        let payload = Bytes::from_static(b"not-json");
        let original_ptr = payload.as_ptr();

        let updated =
            rewrite_body_model_if_required(payload, ProviderFormat::ChatCompletions, "gpt-4o");

        assert_eq!(updated.as_ptr(), original_ptr);
    }
}
