pub mod adapters;
pub mod decisions;
pub mod dedup;
pub mod import;
mod json_repair;
pub mod stream;
pub mod trajectory;
pub mod transform;

pub use adapters::{
    DecisionAdapter, ProviderAdapter, adapter_for_format, adapters, collect_extras,
    decision_adapter_for_format, insert_opt_bool, insert_opt_f64, insert_opt_i64,
    insert_opt_string, insert_opt_value,
};
pub use decisions::{
    DecisionRequestTransform, DecisionResponseContext, transform_decision_request,
    transform_decision_response,
};
pub use dedup::{deduplicate_messages, deduplicate_messages_iter, message_dedup_hash};
pub use import::{
    Span, import_and_deduplicate_messages, import_messages_from_spans,
    import_role_message_from_value, try_import_ai_sdk_prompt_message,
};
pub use json_repair::normalize_json_lone_surrogate_escapes;
pub use stream::{
    ParsedStreamEvent, StreamOutputChunk, StreamTransformSession, parse_stream_event,
};
pub use transform::{
    ParsedJsonBody, RequestTransformResult, ResponseTransformResult, TransformError,
    TransformResult, extract_model, normalize_universal_request_for_target, parse_json,
    parse_json_body, parse_json_value, request_to_universal, response_to_universal,
    sanitize_payload, transform_request, transform_response, transform_stream_chunk,
};
