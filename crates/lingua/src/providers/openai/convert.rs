use crate::error::ConvertError;
use crate::import_parse::{
    non_empty_messages, try_convert_non_empty, try_parse, try_parse_vec_or_single,
};
use crate::providers::openai::generated as openai;
use crate::providers::openai::params::OpenAIResponsesExtrasView;
use crate::providers::openai::tool_discovery;
use crate::serde_json;
use crate::universal::convert::TryFromLLM;
use crate::universal::defaults::{EMPTY_OBJECT_STR, PLACEHOLDER_ID, REFUSAL_TEXT};
use crate::universal::{
    AssistantContent, AssistantContentPart, CacheControl, Message, ProviderOptions,
    TextContentPart, ToolCallArguments, ToolCaller, ToolCallerType, ToolContentPart,
    ToolDiscoveryResultContentPart, ToolDiscoveryResultItem, ToolResultContentPart, UserContent,
    UserContentPart,
};
use crate::util::media::parse_base64_data_url;
use base64::Engine;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

fn openai_arguments_to_string(arguments: openai::Arguments) -> String {
    match arguments {
        openai::Arguments::String(value) => value,
        openai::Arguments::AnythingMap(value) => serde_json::Value::Object(value).to_string(),
    }
}

fn openai_arguments_from_string(arguments: String) -> openai::Arguments {
    openai::Arguments::String(arguments)
}

fn openai_output_to_string(output: openai::Output) -> String {
    match output {
        openai::Output::String(value) => value,
        openai::Output::ComputerScreenshotImage(value) => match serde_json::to_string(&value) {
            Ok(value) => value,
            Err(error) => format!("failed to serialize computer screenshot output: {error}"),
        },
        openai::Output::PurpleInputContentArray(value) => match serde_json::to_string(&value) {
            Ok(value) => value,
            Err(error) => format!("failed to serialize content array output: {error}"),
        },
    }
}

fn openai_output_from_string(output: String) -> openai::Output {
    openai::Output::String(output)
}

fn output_item_arguments_to_input(
    arguments: Option<serde_json::Value>,
) -> Option<openai::Arguments> {
    arguments.map(|value| match value {
        serde_json::Value::String(value) => openai::Arguments::String(value),
        serde_json::Value::Object(value) => openai::Arguments::AnythingMap(value),
        value => openai::Arguments::String(value.to_string()),
    })
}

fn input_item_arguments_to_output(
    arguments: Option<openai::Arguments>,
) -> Option<serde_json::Value> {
    arguments.map(|arguments| match arguments {
        openai::Arguments::String(value) => serde_json::Value::String(value),
        openai::Arguments::AnythingMap(value) => serde_json::Value::Object(value),
    })
}

fn output_item_output_to_input(output: Option<openai::OutputUnion>) -> Option<openai::Output> {
    output.map(|output| match output {
        openai::OutputUnion::String(value) => openai::Output::String(value),
        openai::OutputUnion::ComputerScreenshotImage(value) => {
            openai::Output::ComputerScreenshotImage(value)
        }
        openai::OutputUnion::FluffyInputContentArray(value) => {
            match serde_json::to_value(value).and_then(serde_json::from_value) {
                Ok(value) => openai::Output::PurpleInputContentArray(value),
                Err(error) => openai::Output::String(format!(
                    "failed to convert output content array: {error}"
                )),
            }
        }
    })
}

fn input_item_output_to_output(output: Option<openai::Output>) -> Option<openai::OutputUnion> {
    output.map(|output| match output {
        openai::Output::String(value) => openai::OutputUnion::String(value),
        openai::Output::ComputerScreenshotImage(value) => {
            openai::OutputUnion::ComputerScreenshotImage(value)
        }
        openai::Output::PurpleInputContentArray(value) => {
            match serde_json::to_value(value).and_then(serde_json::from_value) {
                Ok(value) => openai::OutputUnion::FluffyInputContentArray(value),
                Err(error) => openai::OutputUnion::String(format!(
                    "failed to convert input content array: {error}"
                )),
            }
        }
    })
}

fn function_call_item_status_to_string(
    status: openai::FunctionCallItemStatus,
    field: &str,
) -> Result<String, ConvertError> {
    match serde_json::to_value(status).map_err(|e| ConvertError::JsonSerializationFailed {
        field: field.to_string(),
        error: e.to_string(),
    })? {
        serde_json::Value::String(value) => Ok(value),
        value => Err(ConvertError::InvalidEnumValue {
            type_name: "FunctionCallItemStatus",
            value: value.to_string(),
        }),
    }
}

fn merge_reasoning_signature(
    current: &mut Option<String>,
    next: &Option<String>,
) -> Result<(), ConvertError> {
    let Some(next) = next else {
        return Ok(());
    };

    match current {
        Some(current) if current != next => Err(ConvertError::ContentConversionFailed {
            reason: "OpenAI Chat Completions response messages only support one reasoning_signature, but the assistant content contains multiple distinct encrypted tool/reasoning signatures".to_string(),
        }),
        Some(_) => Ok(()),
        None => {
            *current = Some(next.clone());
            Ok(())
        }
    }
}

/// Extended ChatCompletionRequest/ResponseMessage with reasoning support.
///
/// The official OpenAI Chat Completions API doesn't include a `reasoning` field on messages.`                                         
/// With the release of gpt-oss, OpenAI's guidance is to handle reasoning content with                                                 
/// a top-level `reasoning` field. https://cookbook.openai.com/articles/gpt-oss/handle-raw-cot#chat-completions-api                    
///
/// These extension type uses `#[serde(flatten)]` to wrap the generated type while adding
/// the `reasoning` field, keeping generated types faithful to the official spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponseMessageExt {
    #[serde(flatten)]
    pub base: openai::ChatCompletionResponseMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Encrypted reasoning signature for cross-provider roundtrips (e.g., Anthropic's signature)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageExt {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ChatCompletionRequestMessageContentExt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub role: openai::ChatCompletionRequestMessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<openai::ChatCompletionRequestMessageAudio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<openai::ChatCompletionRequestMessageFunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<openai::ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ChatCompletionRequestReasoning>,
    /// Encrypted reasoning signature for cross-provider roundtrips (e.g., Anthropic's signature)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionRequestMessageContentExt {
    Parts(Vec<ChatCompletionRequestMessageContentPartExt>),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartExt {
    #[serde(flatten)]
    pub base: openai::ChatCompletionRequestMessageContentPart,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
}

impl From<ChatCompletionRequestMessageContentExt> for openai::ChatCompletionRequestMessageContent {
    fn from(content: ChatCompletionRequestMessageContentExt) -> Self {
        match content {
            ChatCompletionRequestMessageContentExt::Parts(parts) => {
                openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(
                    parts.into_iter().map(|part| part.base).collect(),
                )
            }
            ChatCompletionRequestMessageContentExt::String(text) => {
                openai::ChatCompletionRequestMessageContent::String(text)
            }
        }
    }
}

impl From<openai::ChatCompletionRequestMessageContent> for ChatCompletionRequestMessageContentExt {
    fn from(content: openai::ChatCompletionRequestMessageContent) -> Self {
        match content {
            openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(
                parts,
            ) => ChatCompletionRequestMessageContentExt::Parts(
                parts.into_iter()
                    .map(|base| ChatCompletionRequestMessageContentPartExt {
                        base,
                        cache_control: None,
                    })
                    .collect(),
            ),
            openai::ChatCompletionRequestMessageContent::String(text) => {
                ChatCompletionRequestMessageContentExt::String(text)
            }
        }
    }
}

impl From<openai::ChatCompletionRequestMessage> for ChatCompletionRequestMessageExt {
    fn from(message: openai::ChatCompletionRequestMessage) -> Self {
        Self {
            content: message.content.map(Into::into),
            name: message.name,
            role: message.role,
            audio: message.audio,
            function_call: message.function_call,
            refusal: message.refusal,
            tool_calls: message.tool_calls,
            tool_call_id: message.tool_call_id,
            cache_control: None,
            reasoning: None,
            reasoning_signature: None,
        }
    }
}

impl From<ChatCompletionRequestMessageExt> for openai::ChatCompletionRequestMessage {
    fn from(message: ChatCompletionRequestMessageExt) -> Self {
        Self {
            content: message.content.map(Into::into),
            name: message.name,
            role: message.role,
            audio: message.audio,
            function_call: message.function_call,
            refusal: message.refusal,
            tool_calls: message.tool_calls,
            tool_call_id: message.tool_call_id,
        }
    }
}

fn cache_control_from_value(cache_control: Option<serde_json::Value>) -> Option<CacheControl> {
    cache_control.and_then(|value| serde_json::from_value(value).ok())
}

fn cache_control_to_value(cache_control: Option<CacheControl>) -> Option<serde_json::Value> {
    cache_control.and_then(|cache_control| serde_json::to_value(cache_control).ok())
}

fn assistant_content_from_parts(content_parts: Vec<AssistantContentPart>) -> AssistantContent {
    if content_parts.is_empty() {
        AssistantContent::String(String::new())
    } else if content_parts.len() == 1 {
        match &content_parts[0] {
            AssistantContentPart::Text(text_part)
                if text_part.cache_control.is_none()
                    && text_part.encrypted_content.is_none()
                    && text_part.provider_options.is_none() =>
            {
                AssistantContent::String(text_part.text.clone())
            }
            _ => AssistantContent::Array(content_parts),
        }
    } else {
        AssistantContent::Array(content_parts)
    }
}

fn chat_completion_content_to_user_content(
    content: Option<ChatCompletionRequestMessageContentExt>,
    cache_control: Option<serde_json::Value>,
) -> Result<UserContent, ConvertError> {
    match content {
        Some(content) => match content {
            ChatCompletionRequestMessageContentExt::String(text) => {
                if cache_control.is_some() {
                    Ok(UserContent::Array(vec![UserContentPart::Text(
                        TextContentPart {
                            text,
                            encrypted_content: None,
                            cache_control: cache_control_from_value(cache_control),
                            provider_options: None,
                        },
                    )]))
                } else {
                    Ok(UserContent::String(text))
                }
            }
            ChatCompletionRequestMessageContentExt::Parts(parts) => {
                let user_parts: Result<Vec<_>, _> =
                    parts.into_iter().map(TryFromLLM::try_from).collect();
                Ok(UserContent::Array(user_parts?))
            }
        },
        None => Err(ConvertError::MissingRequiredField {
            field: "content".to_string(),
        }),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionRequestReasoning {
    String(String),
    Parts(Vec<ChatCompletionRequestReasoningPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequestReasoningPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

impl ChatCompletionRequestReasoning {
    fn into_text(self) -> Option<String> {
        match self {
            Self::String(text) => Some(text),
            Self::Parts(parts) => {
                let mut combined = String::new();
                for part in parts {
                    if let Some(content) = part.content {
                        combined.push_str(&content);
                    }
                }
                if combined.is_empty() {
                    return None;
                }
                Some(combined)
            }
        }
    }
}

/// Helper function to build ToolCallArguments from a JSON value
fn build_tool_arguments(value: &serde_json::Value) -> ToolCallArguments {
    match value.as_object() {
        Some(map) => ToolCallArguments::Valid(map.clone()),
        None => ToolCallArguments::Invalid(value.to_string()),
    }
}

/// Helper to parse an optional field from JSON with proper error handling.
///
/// Returns `Ok(None)` if the field is missing or null, `Ok(Some(value))` if parsing succeeds,
/// or `Err` with a descriptive error if parsing fails.
fn parse_builtin_field<T: serde::de::DeserializeOwned>(
    value: &serde_json::Value,
    field: &str,
    tool_name: &str,
) -> Result<Option<T>, ConvertError> {
    match value.get(field) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => serde_json::from_value(v.clone()).map(Some).map_err(|e| {
            ConvertError::JsonSerializationFailed {
                field: format!("{}.{}", tool_name, field),
                error: e.to_string(),
            }
        }),
        None => Ok(None),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum ResponsesImportKnownType {
    // Some SDK/frontend traces use compatibility shapes (`function_call_result`,
    // camelCase `callId`, JSON-valued `output`/`result`) that are not in the
    // canonical OpenAI schema used to generate `openai::*` types.
    #[serde(rename = "function_call_output", alias = "function_call_result")]
    FunctionCallOutput,
    #[serde(rename = "custom_tool_call_output")]
    CustomToolCallOutput,
    #[serde(rename = "function_call")]
    FunctionCall,
    #[serde(rename = "custom_tool_call")]
    CustomToolCall,
    #[serde(rename = "image_generation_call")]
    ImageGenerationCall,
    #[serde(rename = "program")]
    Program,
    #[serde(rename = "program_output")]
    ProgramOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ResponsesImportItemType {
    Known(ResponsesImportKnownType),
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponsesImportCompatItem {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    item_type: Option<ResponsesImportItemType>,
    #[serde(default, alias = "callId", skip_serializing_if = "Option::is_none")]
    call_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_json_as_string",
        skip_serializing_if = "Option::is_none"
    )]
    output: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_json_as_string",
        skip_serializing_if = "Option::is_none"
    )]
    result: Option<String>,
    code: Option<String>,
    fingerprint: Option<String>,
    status: Option<String>,
    caller: Option<OpenAIProgramCaller>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

fn deserialize_optional_json_as_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(text)) => Ok(Some(text)),
        Some(other) => Ok(Some(other.to_string())),
    }
}

fn normalize_responses_items_for_import(data: &serde_json::Value) -> Option<serde_json::Value> {
    let wrapped;
    let candidate = if data.is_object() {
        wrapped = serde_json::Value::Array(vec![data.clone()]);
        &wrapped
    } else {
        data
    };

    let compat_items =
        serde_json::from_value::<Vec<ResponsesImportCompatItem>>(candidate.clone()).ok()?;
    let normalized = serde_json::to_value(compat_items).ok()?;

    if normalized == *candidate {
        None
    } else {
        Some(normalized)
    }
}

enum OpenAIFilePayload {
    FileData(String),
    FileUrl(String),
}

struct UniversalFilePayload {
    data: serde_json::Value,
    media_type: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct OpenAIFileProviderOptionsView {
    title: Option<String>,
}

fn openai_file_provider_options_view(
    provider_options: &Option<ProviderOptions>,
) -> Option<OpenAIFileProviderOptionsView> {
    provider_options.as_ref().and_then(|opts| {
        serde_json::from_value::<OpenAIFileProviderOptionsView>(serde_json::Value::Object(
            opts.options.clone(),
        ))
        .ok()
    })
}

fn openai_filename_for_file(
    filename: Option<String>,
    media_type: &str,
    provider_options: &Option<ProviderOptions>,
) -> Option<String> {
    if filename.is_some() {
        return filename;
    }

    if let Some(title) =
        openai_file_provider_options_view(provider_options).and_then(|opts| opts.title)
    {
        return Some(title);
    }

    Some(match media_type {
        "text/plain" => "document.txt".to_string(),
        "application/pdf" => "document.pdf".to_string(),
        _ => "document".to_string(),
    })
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct OpenAIToolCallProviderOptionsView {
    namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct OpenAIProgramCaller {
    #[serde(rename = "type")]
    caller_type: OpenAIProgramCallerType,
    caller_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OpenAIProgramCallerType {
    Program,
}

impl From<OpenAIProgramCaller> for ToolCaller {
    fn from(caller: OpenAIProgramCaller) -> Self {
        let caller_type = match caller.caller_type {
            OpenAIProgramCallerType::Program => ToolCallerType::Program,
        };
        Self {
            caller_type,
            caller_id: Some(caller.caller_id),
        }
    }
}

impl From<openai::DirectToolCallCallerType> for ToolCallerType {
    fn from(caller_type: openai::DirectToolCallCallerType) -> Self {
        match caller_type {
            openai::DirectToolCallCallerType::Direct => ToolCallerType::Direct,
            openai::DirectToolCallCallerType::Program => ToolCallerType::Program,
        }
    }
}

impl From<ToolCallerType> for openai::DirectToolCallCallerType {
    fn from(caller_type: ToolCallerType) -> Self {
        match caller_type {
            ToolCallerType::Direct => openai::DirectToolCallCallerType::Direct,
            ToolCallerType::Program => openai::DirectToolCallCallerType::Program,
        }
    }
}

impl From<openai::InputItemDirectToolCallCaller> for ToolCaller {
    fn from(caller: openai::InputItemDirectToolCallCaller) -> Self {
        Self {
            caller_type: caller.direct_tool_call_caller_type.into(),
            caller_id: caller.caller_id,
        }
    }
}

impl From<openai::OutputItemDirectToolCallCaller> for ToolCaller {
    fn from(caller: openai::OutputItemDirectToolCallCaller) -> Self {
        Self {
            caller_type: caller.direct_tool_call_caller_type.into(),
            caller_id: caller.caller_id,
        }
    }
}

impl From<ToolCaller> for openai::InputItemDirectToolCallCaller {
    fn from(caller: ToolCaller) -> Self {
        Self {
            direct_tool_call_caller_type: caller.caller_type.into(),
            caller_id: caller.caller_id,
        }
    }
}

impl From<ToolCaller> for openai::OutputItemDirectToolCallCaller {
    fn from(caller: ToolCaller) -> Self {
        Self {
            direct_tool_call_caller_type: caller.caller_type.into(),
            caller_id: caller.caller_id,
        }
    }
}

fn openai_tool_call_provider_options_view(
    provider_options: &Option<ProviderOptions>,
) -> Option<OpenAIToolCallProviderOptionsView> {
    provider_options.as_ref().and_then(|opts| {
        serde_json::from_value::<OpenAIToolCallProviderOptionsView>(serde_json::Value::Object(
            opts.options.clone(),
        ))
        .ok()
    })
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct OpenAITextProviderOptionsView {
    phase: Option<openai::MessagePhase>,
}

fn openai_text_provider_options_view(
    provider_options: &Option<ProviderOptions>,
) -> Result<OpenAITextProviderOptionsView, ConvertError> {
    let Some(provider_options) = provider_options else {
        return Ok(OpenAITextProviderOptionsView::default());
    };
    serde_json::from_value(serde_json::Value::Object(provider_options.options.clone())).map_err(
        |error| ConvertError::ContentConversionFailed {
            reason: format!("invalid OpenAI text provider options: {error}"),
        },
    )
}

fn provider_options_from_openai_tool_call(namespace: Option<String>) -> Option<ProviderOptions> {
    let namespace = namespace?;
    let mut options = serde_json::Map::new();
    options.insert(
        "namespace".to_string(),
        serde_json::Value::String(namespace),
    );
    Some(ProviderOptions { options })
}

fn non_completed_function_call_status_to_string(
    status: Option<openai::FunctionCallItemStatus>,
    field: &str,
) -> Result<Option<String>, ConvertError> {
    status
        .filter(|status| *status != openai::FunctionCallItemStatus::Completed)
        .map(|status| function_call_item_status_to_string(status, field))
        .transpose()
}

fn function_call_item_status_from_string(
    status: &str,
    field: &str,
) -> Result<openai::FunctionCallItemStatus, ConvertError> {
    serde_json::from_value::<openai::FunctionCallItemStatus>(serde_json::Value::String(
        status.to_string(),
    ))
    .map_err(|_| ConvertError::InvalidEnumValue {
        type_name: "FunctionCallItemStatus",
        value: format!("{field}: {status}"),
    })
}

#[derive(Debug, Clone)]
struct ResponsesToolCallInfo {
    tool_call_id: String,
    tool_name: String,
    arguments: ToolCallArguments,
    namespace: Option<String>,
    status: Option<String>,
    caller: Option<ToolCaller>,
    provider_executed: Option<bool>,
}

#[derive(Debug, Clone)]
struct ResponsesDiscoveryCallInfo {
    tool_call_id: String,
    query: Option<String>,
    arguments: Option<serde_json::Value>,
    status: Option<String>,
    execution: Option<String>,
}

enum ResponsesSequencedInputItem {
    ToolCall(ResponsesToolCallInfo),
    DiscoveryCall(ResponsesDiscoveryCallInfo),
    Item(Box<openai::InputItem>),
}

fn responses_tool_values_from_universal_tools(
    tools: &[crate::universal::UniversalTool],
) -> Result<Vec<serde_json::Value>, ConvertError> {
    tools.iter().map(|tool| tool.to_responses_value()).collect()
}

fn responses_tool_values_from_discovery_items(
    tools: &[ToolDiscoveryResultItem],
) -> Result<Vec<serde_json::Value>, ConvertError> {
    tools
        .iter()
        .map(|item| {
            if let Some(tool) = &item.tool {
                tool.to_responses_value()
            } else {
                Ok(serde_json::json!({
                    "type": "function",
                    "name": item.tool_name,
                    "parameters": {"type": "object"}
                }))
            }
        })
        .collect()
}

fn additional_tools_messages(messages: &[Message]) -> Vec<&[crate::universal::UniversalTool]> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::AdditionalTools { tools, .. } => Some(tools.as_slice()),
            _ => None,
        })
        .collect()
}

fn tool_discovery_results(messages: &[Message]) -> Vec<&ToolDiscoveryResultContentPart> {
    messages
        .iter()
        .flat_map(|message| match message {
            Message::Tool { content } => content
                .iter()
                .filter_map(|part| match part {
                    ToolContentPart::ToolDiscoveryResult(result) => Some(result),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
}

pub(crate) fn responses_input_values_from_universal_context(
    input_items: &[openai::InputItem],
    messages: &[Message],
) -> Result<Vec<serde_json::Value>, ConvertError> {
    let mut values = Vec::with_capacity(input_items.len());
    let mut additional_tools = additional_tools_messages(messages).into_iter();
    let mut discovery_results = tool_discovery_results(messages).into_iter();
    for item in input_items {
        let mut value =
            serde_json::to_value(item).map_err(|e| ConvertError::JsonSerializationFailed {
                field: "Responses input item".to_string(),
                error: e.to_string(),
            })?;
        match item.input_item_type {
            Some(openai::InputItemType::AdditionalTools) => {
                if let Some(tools) = additional_tools.next() {
                    value["tools"] = serde_json::Value::Array(
                        responses_tool_values_from_universal_tools(tools)?,
                    );
                }
            }
            Some(openai::InputItemType::ToolSearchOutput) => {
                if let Some(result) = discovery_results.next() {
                    value["tools"] = serde_json::Value::Array(
                        responses_tool_values_from_discovery_items(&result.tools)?,
                    );
                }
            }
            _ => {}
        }
        values.push(value);
    }
    Ok(values)
}

pub(crate) fn responses_output_values_from_universal_context(
    output_items: &[openai::OutputItem],
    messages: &[Message],
) -> Result<Vec<serde_json::Value>, ConvertError> {
    let mut values = Vec::with_capacity(output_items.len());
    let mut additional_tools = additional_tools_messages(messages).into_iter();
    let mut discovery_results = tool_discovery_results(messages).into_iter();
    for item in output_items {
        let mut value =
            serde_json::to_value(item).map_err(|e| ConvertError::JsonSerializationFailed {
                field: "Responses output item".to_string(),
                error: e.to_string(),
            })?;
        match item.output_item_type {
            Some(openai::OutputItemType::AdditionalTools) => {
                if let Some(tools) = additional_tools.next() {
                    value["tools"] = serde_json::Value::Array(
                        responses_tool_values_from_universal_tools(tools)?,
                    );
                }
            }
            Some(openai::OutputItemType::ToolSearchOutput) => {
                if let Some(result) = discovery_results.next() {
                    value["tools"] = serde_json::Value::Array(
                        responses_tool_values_from_discovery_items(&result.tools)?,
                    );
                }
            }
            _ => {}
        }
        values.push(value);
    }
    Ok(values)
}

fn openai_media_type_from_reference(filename: Option<&str>, file_url: Option<&str>) -> String {
    let extension = filename
        .and_then(|name| name.rsplit('.').next().map(str::to_string))
        .or_else(|| {
            file_url
                .and_then(|url| url.rsplit('/').next())
                .and_then(|segment| segment.split('?').next())
                .and_then(|name| name.rsplit('.').next())
                .map(str::to_string)
        });

    match extension.as_deref() {
        Some("txt") => "text/plain".to_string(),
        Some("pdf") => "application/pdf".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

fn openai_file_payload_from_data(
    data: serde_json::Value,
    media_type: &str,
) -> Result<OpenAIFilePayload, ConvertError> {
    let data = match data {
        serde_json::Value::String(value) => value,
        other => {
            return Err(ConvertError::UnsupportedInputType {
                type_info: format!(
                    "File data must be string-backed for OpenAI, got: {:?}",
                    other
                ),
            })
        }
    };

    if let Some(block) = parse_base64_data_url(&data) {
        return Ok(OpenAIFilePayload::FileData(format!(
            "data:{};base64,{}",
            block.media_type, block.data
        )));
    }

    if data.starts_with("http://") || data.starts_with("https://") {
        return Ok(OpenAIFilePayload::FileUrl(data));
    }

    Ok(OpenAIFilePayload::FileData(format!(
        "data:{};base64,{}",
        media_type,
        base64::engine::general_purpose::STANDARD.encode(data.as_bytes())
    )))
}

fn universal_file_payload_from_openai(
    file_data: Option<String>,
    file_url: Option<String>,
    file_id: Option<String>,
    filename: Option<String>,
) -> Result<(UniversalFilePayload, Option<String>), ConvertError> {
    if let Some(file_id) = file_id {
        return Err(ConvertError::UnsupportedInputType {
            type_info: format!("OpenAI file_id inputs are not supported: {}", file_id),
        });
    }

    let media_type = openai_media_type_from_reference(filename.as_deref(), file_url.as_deref());

    if let Some(file_url) = file_url {
        return Ok((
            UniversalFilePayload {
                data: serde_json::Value::String(file_url),
                media_type,
            },
            filename,
        ));
    }

    if let Some(file_data) = file_data {
        if let Some(block) = parse_base64_data_url(&file_data) {
            let decoded_text = base64::engine::general_purpose::STANDARD
                .decode(&block.data)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok());
            let has_known_extension = filename
                .as_deref()
                .and_then(|name| name.rsplit('.').next())
                .map(|ext| matches!(ext, "txt" | "pdf"))
                .unwrap_or(false);
            let filename = if decoded_text.is_some() && !has_known_extension {
                None
            } else {
                filename
            };
            let media_type = if decoded_text.is_some() {
                "text/plain".to_string()
            } else {
                block.media_type
            };
            let data = decoded_text
                .map(serde_json::Value::String)
                .unwrap_or_else(|| serde_json::Value::String(block.data));

            return Ok((UniversalFilePayload { data, media_type }, filename));
        }

        let decoded_text = base64::engine::general_purpose::STANDARD
            .decode(&file_data)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok());
        let has_known_extension = filename
            .as_deref()
            .and_then(|name| name.rsplit('.').next())
            .map(|ext| matches!(ext, "txt" | "pdf"))
            .unwrap_or(false);
        let filename = if decoded_text.is_some() && !has_known_extension {
            None
        } else {
            filename
        };
        let media_type = if decoded_text.is_some() {
            "text/plain".to_string()
        } else {
            media_type
        };
        let data = decoded_text
            .map(serde_json::Value::String)
            .unwrap_or_else(|| serde_json::Value::String(file_data));

        return Ok((UniversalFilePayload { data, media_type }, filename));
    }

    Err(ConvertError::MissingRequiredField {
        field: "file_data|file_url|file_id".to_string(),
    })
}

fn is_reasoning_only_assistant_message(message: &Message) -> bool {
    match message {
        Message::Assistant {
            content: AssistantContent::Array(parts),
            ..
        } => {
            !parts.is_empty()
                && parts
                    .iter()
                    .all(|part| matches!(part, AssistantContentPart::Reasoning { .. }))
        }
        _ => false,
    }
}

fn merge_adjacent_reasoning_assistant_messages(messages: Vec<Message>) -> Vec<Message> {
    let mut merged = Vec::with_capacity(messages.len());

    for message in messages {
        let should_merge = matches!(merged.last(), Some(prev) if is_reasoning_only_assistant_message(prev))
            && matches!(message, Message::Assistant { .. });

        if !should_merge {
            merged.push(message);
            continue;
        }

        let Some(previous) = merged.pop() else {
            merged.push(message);
            continue;
        };

        let Message::Assistant {
            content: AssistantContent::Array(reasoning_parts),
            id: reasoning_id,
        } = previous
        else {
            merged.push(previous);
            merged.push(message);
            continue;
        };

        let Message::Assistant {
            content: next_content,
            id: next_id,
        } = message
        else {
            merged.push(Message::Assistant {
                content: AssistantContent::Array(reasoning_parts),
                id: reasoning_id,
            });
            merged.push(message);
            continue;
        };

        let mut combined_parts = reasoning_parts;
        match next_content {
            AssistantContent::Array(parts) => combined_parts.extend(parts),
            AssistantContent::String(text) => {
                combined_parts.push(AssistantContentPart::Text(TextContentPart {
                    text,
                    encrypted_content: None,
                    cache_control: None,
                    provider_options: None,
                }));
            }
        }

        merged.push(Message::Assistant {
            content: AssistantContent::Array(combined_parts),
            id: next_id.or(reasoning_id),
        });
    }

    merged
}

fn try_from_responses_items_candidate(candidate: &serde_json::Value) -> Option<Vec<Message>> {
    if let Some(provider_messages) = try_parse_vec_or_single::<openai::InputItem>(candidate) {
        if let Some(messages) = try_convert_non_empty(provider_messages) {
            return non_empty_messages(merge_adjacent_reasoning_assistant_messages(messages));
        }
    }

    if let Some(provider_messages) = try_parse_vec_or_single::<openai::OutputItem>(candidate) {
        if let Some(messages) = try_convert_non_empty(provider_messages) {
            return non_empty_messages(merge_adjacent_reasoning_assistant_messages(messages));
        }
    }

    None
}

pub(crate) fn try_parse_responses_items_for_import(
    data: &serde_json::Value,
) -> Option<Vec<Message>> {
    if let Some(messages) = try_from_responses_items_candidate(data) {
        return Some(messages);
    }

    let normalized = normalize_responses_items_for_import(data)?;
    try_from_responses_items_candidate(&normalized)
}

fn try_messages_from_openai_instructions(input: openai::Instructions) -> Option<Vec<Message>> {
    match input {
        openai::Instructions::InputItemArray(items) => {
            let messages = try_convert_non_empty(items)?;
            non_empty_messages(merge_adjacent_reasoning_assistant_messages(messages))
        }
        openai::Instructions::String(text) => Some(vec![Message::User {
            content: UserContent::String(text),
        }]),
    }
}

fn extract_instructions_from_openai_metadata_value(metadata: &serde_json::Value) -> Option<String> {
    let typed = match metadata {
        serde_json::Value::String(metadata_json) => {
            let parsed = serde_json::from_str::<serde_json::Value>(metadata_json).ok()?;
            serde_json::from_value::<OpenAIResponsesExtrasView>(parsed).ok()?
        }
        _ => serde_json::from_value::<OpenAIResponsesExtrasView>(metadata.clone()).ok()?,
    };
    typed.instructions
}

pub(crate) fn try_system_message_from_openai_metadata(
    metadata: &serde_json::Value,
) -> Option<Message> {
    let instructions = extract_instructions_from_openai_metadata_value(metadata)?;
    if instructions.is_empty() {
        return None;
    }
    Some(Message::System {
        content: UserContent::String(instructions),
    })
}

pub(crate) fn try_parse_openai_for_import(data: &serde_json::Value) -> Option<Vec<Message>> {
    // Prefer chat-completions request messages before Responses InputItem parsing.
    // Chat-completions arrays can deserialize as InputItems, but that path drops
    // assistant `tool_calls` arguments and is lossy for import.
    if let Some(provider_messages) =
        try_parse_vec_or_single::<ChatCompletionRequestMessageExt>(data)
    {
        if let Some(messages) = try_convert_non_empty(provider_messages) {
            return non_empty_messages(merge_adjacent_reasoning_assistant_messages(messages));
        }
    }

    if let Some(messages) = try_parse_responses_items_for_import(data) {
        return Some(messages);
    }

    if let Some(request) = try_parse::<openai::CreateResponseClass>(data) {
        if let Some(input) = request.input {
            if let Some(messages) = try_messages_from_openai_instructions(input) {
                return Some(messages);
            }
        }
    }

    if let Some(response) = try_parse::<openai::TheResponseObject>(data) {
        let messages = try_convert_non_empty(response.output)?;
        return non_empty_messages(merge_adjacent_reasoning_assistant_messages(messages));
    }

    None
}

/// Convert OpenAI InputItem collection to universal Message collection
/// This handles OpenAI-specific logic for combining or transforming multiple items
impl TryFromLLM<Vec<openai::InputItem>> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(inputs: Vec<openai::InputItem>) -> Result<Self, Self::Error> {
        let mut result = Vec::new();
        let mut last_tool_search_call_id: Option<String> = None;
        for mut input in inputs {
            let pending_tool_search_call_id = last_tool_search_call_id.take();
            match input.input_item_type {
                // Built-in tool calls - convert to ToolCall with provider_executed: true
                Some(openai::InputItemType::WebSearchCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "web_search".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": input.action,
                            "queries": input.queries,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: input.caller.map(Into::into),
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::CodeInterpreterCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "code_interpreter".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "code": input.code,
                            "container_id": input.container_id,
                            "outputs": input.outputs,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: input.caller.map(Into::into),
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::FileSearchCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "file_search".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "queries": input.queries,
                            "results": input.results,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: input.caller.map(Into::into),
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::ComputerCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "computer".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": input.action,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::ImageGenerationCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "image_generation".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "result": input.result,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::LocalShellCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "local_shell".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": input.action,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::McpCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "mcp_call".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "server_label": input.server_label,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::McpListTools) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "mcp_list_tools".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "server_label": input.server_label,
                            "tools": input.tools,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::McpApprovalRequest) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "mcp_approval_request".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::ToolSearchCall) => {
                    last_tool_search_call_id = input.call_id.clone().or_else(|| input.id.clone());
                    result.push(tool_discovery::message_from_input_call(input)?);
                }
                Some(openai::InputItemType::ToolSearchOutput) => {
                    if input.call_id.is_none() {
                        input.call_id = pending_tool_search_call_id;
                    }
                    result.push(tool_discovery::message_from_input_output(input)?);
                }
                Some(openai::InputItemType::ItemReference) => {
                    // Full Responses request conversion extracts these into
                    // UniversalParams::conversation_reference before message conversion.
                    continue;
                }
                Some(openai::InputItemType::AdditionalTools) => {
                    result.push(tool_discovery::message_from_input_additional_tools(input)?);
                }
                Some(openai::InputItemType::Reasoning) => {
                    let mut summaries = vec![];
                    let mut first = true;
                    for summary in input.summary.unwrap_or_default() {
                        summaries.push(AssistantContentPart::Reasoning {
                            text: summary.text,
                            // OpenAI returns encrypted content on the message level, but may
                            // return multiple summary parts. To keep it simple, we just match this
                            // convention by putting the encrypted content on the first part.
                            encrypted_content: if first {
                                first = false;
                                input.encrypted_content.take()
                            } else {
                                None
                            },
                        });
                    }

                    if summaries.is_empty() {
                        // Handle case where there are no summary parts (empty reasoning). This way
                        // we stil get the encrypted content and make it clear that there was a
                        // reasoning step.
                        summaries.push(AssistantContentPart::Reasoning {
                            text: "".to_string(),
                            encrypted_content: input.encrypted_content.take(),
                        });
                    }

                    result.push(Message::Assistant {
                        content: AssistantContent::Array(summaries),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::Program) => {
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![AssistantContentPart::Program {
                            id: input.id.clone(),
                            call_id: input.call_id.ok_or_else(|| {
                                ConvertError::MissingRequiredField {
                                    field: "program call_id".to_string(),
                                }
                            })?,
                            code: input
                                .code
                                .ok_or_else(|| ConvertError::MissingRequiredField {
                                    field: "program code".to_string(),
                                })?,
                            fingerprint: input.fingerprint,
                        }]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::ProgramOutput) => {
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![
                            AssistantContentPart::ProgramOutput {
                                id: input.id.clone(),
                                call_id: input.call_id.ok_or_else(|| {
                                    ConvertError::MissingRequiredField {
                                        field: "program_output call_id".to_string(),
                                    }
                                })?,
                                result: input.result.ok_or_else(|| {
                                    ConvertError::MissingRequiredField {
                                        field: "program_output result".to_string(),
                                    }
                                })?,
                                status: input
                                    .status
                                    .map(|status| {
                                        function_call_item_status_to_string(
                                            status,
                                            "program_output status",
                                        )
                                    })
                                    .transpose()?
                                    .ok_or_else(|| ConvertError::MissingRequiredField {
                                        field: "program_output status".to_string(),
                                    })?,
                            },
                        ]),
                        id: input.id,
                    });
                }
                item_type @ (Some(openai::InputItemType::FunctionCall)
                | Some(openai::InputItemType::CustomToolCall)) => {
                    // Function calls are converted to tool calls in assistant messages
                    let tool_call_id =
                        input
                            .call_id
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call call_id".to_string(),
                            })?;
                    let tool_name =
                        input
                            .name
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call name".to_string(),
                            })?;
                    let arguments = match item_type {
                        Some(openai::InputItemType::CustomToolCall) => {
                            ToolCallArguments::Custom(input.input.ok_or_else(|| {
                                ConvertError::MissingRequiredField {
                                    field: "custom tool call input".to_string(),
                                }
                            })?)
                        }
                        _ => input
                            .arguments
                            .map(openai_arguments_to_string)
                            .unwrap_or_else(|| EMPTY_OBJECT_STR.to_string())
                            .into(),
                    };

                    let caller = input.caller;
                    let status = non_completed_function_call_status_to_string(
                        input.status,
                        "Responses function call status",
                    )?;
                    let tool_call_part = AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments,
                        encrypted_content: None,
                        provider_options: provider_options_from_openai_tool_call(input.namespace),
                        status,
                        caller: caller.map(Into::into),
                        provider_executed: None,
                    };

                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call_part]),
                        id: input.id.clone(),
                    });
                }
                item_type @ (Some(openai::InputItemType::FunctionCallOutput)
                | Some(openai::InputItemType::CustomToolCallOutput)) => {
                    // Function call outputs are converted to tool messages
                    let tool_call_id =
                        input
                            .call_id
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call output call_id".to_string(),
                            })?;

                    let output = input
                        .output
                        .map(openai_output_to_string)
                        .unwrap_or_else(|| "".to_string());

                    let output_value =
                        serde_json::from_str(&output).unwrap_or(serde_json::Value::String(output));

                    let tool_result = ToolResultContentPart {
                        tool_call_id,
                        tool_name: input.name.clone().unwrap_or_default(),
                        output: output_value,
                        custom_tool_call: (item_type
                            == Some(openai::InputItemType::CustomToolCallOutput))
                        .then_some(true),
                        caller: input.caller.map(Into::into),
                        provider_options: None,
                    };

                    result.push(Message::Tool {
                        content: vec![ToolContentPart::ToolResult(tool_result)],
                    });
                }
                _ => {
                    let role = input
                        .role
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "role".to_string(),
                        })?;

                    let content =
                        input
                            .content
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "content".to_string(),
                            })?;

                    result.push(match role {
                        openai::InputItemRole::System => Message::System {
                            content: TryFromLLM::try_from(content)?,
                        },
                        openai::InputItemRole::Developer => Message::Developer {
                            content: TryFromLLM::try_from(content)?,
                        },
                        openai::InputItemRole::User => Message::User {
                            content: TryFromLLM::try_from(content)?,
                        },
                        openai::InputItemRole::Assistant => {
                            let content = preserve_responses_message_phase(
                                TryFromLLM::try_from(content)?,
                                input.phase,
                            )?;
                            Message::Assistant {
                                id: input.id,
                                content,
                            }
                        }
                    });
                }
            };
        }

        Ok(result)
    }
}

fn preserve_responses_message_phase(
    content: AssistantContent,
    phase: Option<openai::MessagePhase>,
) -> Result<AssistantContent, ConvertError> {
    let Some(phase) = phase else {
        return Ok(content);
    };
    let phase =
        serde_json::to_value(phase).map_err(|error| ConvertError::JsonSerializationFailed {
            field: "phase".to_string(),
            error: error.to_string(),
        })?;

    match content {
        AssistantContent::String(text) => {
            let mut options = serde_json::Map::new();
            options.insert("phase".to_string(), phase);
            Ok(AssistantContent::Array(vec![AssistantContentPart::Text(
                TextContentPart {
                    text,
                    encrypted_content: None,
                    cache_control: None,
                    provider_options: Some(ProviderOptions { options }),
                },
            )]))
        }
        AssistantContent::Array(mut parts) => {
            let text_part = parts
                .iter_mut()
                .find_map(|part| match part {
                    AssistantContentPart::Text(text_part) => Some(text_part),
                    _ => None,
                })
                .ok_or_else(|| ConvertError::UnsupportedMapping {
                    from: "Responses message phase without text content".to_string(),
                    to: "universal assistant content",
                })?;
            text_part
                .provider_options
                .get_or_insert_with(|| ProviderOptions {
                    options: serde_json::Map::new(),
                })
                .options
                .insert("phase".to_string(), phase);
            Ok(AssistantContent::Array(parts))
        }
    }
}

impl TryFromLLM<openai::InputItemContent> for UserContent {
    type Error = ConvertError;

    fn try_from(contents: openai::InputItemContent) -> Result<Self, Self::Error> {
        Ok(match contents {
            openai::InputItemContent::String(text) => UserContent::String(text),
            openai::InputItemContent::InputContentArray(parts) => {
                UserContent::Array(TryFromLLM::try_from(parts)?)
            }
        })
    }
}

impl TryFromLLM<openai::InputContent> for UserContentPart {
    type Error = ConvertError;

    fn try_from(value: openai::InputContent) -> Result<Self, Self::Error> {
        Ok(match value.input_content_type {
            openai::InputItemContentListType::InputText
            | openai::InputItemContentListType::OutputText => {
                UserContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "text".to_string(),
                        })?,
                    encrypted_content: None,
                    cache_control: None,
                    provider_options: None,
                })
            }
            // TODO: ToolCall and ToolResult content types - not yet implemented in generated types
            openai::InputItemContentListType::InputImage => {
                // Extract image URL from the InputContent
                let image_url =
                    value
                        .image_url
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "image_url".to_string(),
                        })?;

                // Preserve detail in provider_options
                let provider_options = if let Some(detail) = &value.detail {
                    let mut options = serde_json::Map::new();
                    options.insert(
                        "detail".to_string(),
                        serde_json::to_value(detail).map_err(|e| {
                            ConvertError::JsonSerializationFailed {
                                field: "detail".to_string(),
                                error: e.to_string(),
                            }
                        })?,
                    );
                    Some(crate::universal::message::ProviderOptions { options })
                } else {
                    None
                };

                // Parse data URLs to extract raw base64, keep HTTP URLs as-is
                let (image_data, media_type) =
                    if let Some(block) = parse_base64_data_url(&image_url) {
                        // Data URL: extract raw base64 and media type
                        (block.data, Some(block.media_type))
                    } else {
                        // HTTP URL or other: keep as-is with default media type
                        (image_url.clone(), Some("image/jpeg".to_string()))
                    };

                UserContentPart::Image {
                    image: serde_json::Value::String(image_data),
                    media_type,
                    provider_options,
                }
            }
            openai::InputItemContentListType::InputAudio => {
                // Handle audio input if needed in the future
                return Err(ConvertError::UnsupportedInputType {
                    type_info: "InputAudio content type".to_string(),
                });
            }
            openai::InputItemContentListType::InputFile => {
                let (payload, filename) = universal_file_payload_from_openai(
                    value.file_data,
                    value.file_url,
                    value.file_id,
                    value.filename,
                )?;
                UserContentPart::File {
                    data: payload.data,
                    filename,
                    media_type: payload.media_type,
                    provider_options: None,
                }
            }
            openai::InputItemContentListType::ReasoningText => {
                // Handle reasoning text - treat as regular text for now
                UserContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "text".to_string(),
                        })?,
                    encrypted_content: None,
                    cache_control: None,
                    provider_options: None,
                })
            }
            openai::InputItemContentListType::Refusal => {
                // Handle refusal - treat as regular text for now
                UserContentPart::Text(TextContentPart {
                    text: value.text.unwrap_or_else(|| REFUSAL_TEXT.to_string()),
                    encrypted_content: None,
                    cache_control: None,
                    provider_options: None,
                })
            }
        })
    }
}

impl TryFromLLM<openai::InputItemContent> for AssistantContent {
    type Error = ConvertError;

    fn try_from(contents: openai::InputItemContent) -> Result<Self, Self::Error> {
        Ok(match contents {
            openai::InputItemContent::String(text) => AssistantContent::String(text),
            openai::InputItemContent::InputContentArray(parts) => {
                AssistantContent::Array(TryFromLLM::try_from(parts)?)
            }
        })
    }
}

// Add reverse conversions for the reciprocal pattern

impl TryFromLLM<UserContent> for openai::InputItemContent {
    type Error = ConvertError;

    fn try_from(content: UserContent) -> Result<Self, Self::Error> {
        Ok(match content {
            UserContent::String(text) => openai::InputItemContent::String(text),
            UserContent::Array(parts) => {
                let input_parts: Result<Vec<_>, _> =
                    parts.into_iter().map(TryFromLLM::try_from).collect();
                openai::InputItemContent::InputContentArray(input_parts?)
            }
        })
    }
}

impl TryFromLLM<UserContentPart> for openai::InputContent {
    type Error = ConvertError;

    fn try_from(part: UserContentPart) -> Result<Self, Self::Error> {
        Ok(match part {
            UserContentPart::Text(text_part) => openai::InputContent {
                input_content_type: openai::InputItemContentListType::InputText,
                text: Some(text_part.text),
                ..Default::default()
            },
            UserContentPart::Image {
                image,
                media_type,
                provider_options,
            } => {
                let image_str = match image {
                    serde_json::Value::String(url) => url,
                    _ => {
                        return Err(ConvertError::UnsupportedInputType {
                            type_info: format!("Image type must be string URL, got: {:?}", image),
                        })
                    }
                };

                // If we have raw base64 data (not a URL) and media_type, create a proper data URL
                let image_url = if !image_str.starts_with("data:")
                    && !image_str.starts_with("http://")
                    && !image_str.starts_with("https://")
                {
                    // Assume raw base64 data - create data URL with media_type
                    let mt = media_type.as_deref().unwrap_or("image/jpeg");
                    format!("data:{};base64,{}", mt, image_str)
                } else {
                    image_str
                };

                // Extract detail from provider_options if present
                let detail = provider_options
                    .as_ref()
                    .and_then(|opts| opts.options.get("detail"))
                    .and_then(|detail_val| serde_json::from_value(detail_val.clone()).ok());

                openai::InputContent {
                    input_content_type: openai::InputItemContentListType::InputImage,
                    image_url: Some(image_url),
                    detail,
                    ..Default::default()
                }
            }
            UserContentPart::File {
                data,
                filename,
                media_type,
                provider_options,
            } => match openai_file_payload_from_data(data, &media_type)? {
                OpenAIFilePayload::FileUrl(file_url) => openai::InputContent {
                    input_content_type: openai::InputItemContentListType::InputFile,
                    file_url: Some(file_url),
                    filename,
                    ..Default::default()
                },
                OpenAIFilePayload::FileData(file_data) => {
                    let filename =
                        openai_filename_for_file(filename, &media_type, &provider_options);

                    openai::InputContent {
                        input_content_type: openai::InputItemContentListType::InputFile,
                        file_data: Some(file_data),
                        filename,
                        ..Default::default()
                    }
                }
            },
        })
    }
}

impl Default for openai::InputContent {
    fn default() -> Self {
        Self {
            prompt_cache_breakpoint: None,
            text: None,
            input_content_type: openai::InputItemContentListType::InputText,
            detail: None,
            file_id: None,
            image_url: None,
            file_data: None,
            file_url: None,
            filename: None,
            annotations: None,
            logprobs: None,
            refusal: None,
        }
    }
}

impl TryFromLLM<AssistantContent> for openai::InputItemContent {
    type Error = ConvertError;

    fn try_from(content: AssistantContent) -> Result<Self, Self::Error> {
        Ok(match content {
            AssistantContent::String(text) => openai::InputItemContent::String(text),
            AssistantContent::Array(parts) => {
                let input_parts: Result<Vec<_>, _> =
                    parts.into_iter().map(TryFromLLM::try_from).collect();
                openai::InputItemContent::InputContentArray(input_parts?)
            }
        })
    }
}

impl TryFromLLM<AssistantContentPart> for openai::InputContent {
    type Error = ConvertError;

    fn try_from(part: AssistantContentPart) -> Result<Self, Self::Error> {
        Ok(match part {
            AssistantContentPart::Text(text_part) => {
                // Extract annotations and logprobs from provider_options
                let annotations = text_part
                    .provider_options
                    .as_ref()
                    .and_then(|opts| opts.options.get("annotations"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let logprobs = text_part
                    .provider_options
                    .as_ref()
                    .and_then(|opts| opts.options.get("logprobs"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                openai::InputContent {
                    input_content_type: openai::InputItemContentListType::OutputText,
                    text: Some(text_part.text),
                    annotations: Some(annotations),
                    logprobs: Some(logprobs),
                    ..Default::default()
                }
            }
            AssistantContentPart::ToolCall {
                tool_call_id: _,
                tool_name: _,
                arguments,
                ..
            } => openai::InputContent {
                input_content_type: openai::InputItemContentListType::OutputText,
                text: Some(format!("{}", arguments)),
                annotations: Some(vec![]),
                logprobs: Some(vec![]),
                ..Default::default()
            },
            AssistantContentPart::ToolDiscoveryCall { .. } => {
                return Err(ConvertError::UnsupportedInputType {
                    type_info: "AssistantContentPart::ToolDiscoveryCall must be converted as a Responses input item".to_string(),
                })
            }
            AssistantContentPart::Program { .. } | AssistantContentPart::ProgramOutput { .. } => {
                return Err(ConvertError::UnsupportedInputType {
                    type_info: "Programmatic tool calling items must be converted as Responses input items".to_string(),
                })
            }
            AssistantContentPart::Reasoning {
                text,
                encrypted_content: _,
            } => {
                // Convert reasoning back to reasoning text content
                openai::InputContent {
                    input_content_type: openai::InputItemContentListType::ReasoningText,
                    text: Some(text),
                    annotations: Some(vec![]),
                    logprobs: Some(vec![]),
                    ..Default::default()
                }
            }
            AssistantContentPart::ToolResult {
                tool_call_id: _,
                tool_name,
                output,
                ..
            } => {
                // Check for web search tool result marker from Anthropic
                let is_web_search = tool_name == "web_search"
                    || output.get("anthropic_type").and_then(|v| v.as_str())
                        == Some("web_search_tool_result");

                if is_web_search {
                    // Convert web search results to text representation for InputContent
                    // Extract search results content for display
                    let text = match serde_json::to_string(&output) {
                        Ok(text) => text,
                        Err(error) => format!("failed to serialize web search output: {error}"),
                    };
                    openai::InputContent {
                        input_content_type: openai::InputItemContentListType::OutputText,
                        text: Some(text),
                        annotations: Some(vec![]),
                        logprobs: Some(vec![]),
                        ..Default::default()
                    }
                } else {
                    return Err(ConvertError::UnsupportedInputType {
                        type_info: format!(
                            "AssistantContentPart::ToolResult for tool: {}",
                            tool_name
                        ),
                    });
                }
            }
            AssistantContentPart::File { media_type, .. } => {
                return Err(ConvertError::UnsupportedInputType {
                    type_info: format!(
                        "AssistantContentPart::File (media_type: {}) is not supported by OpenAI",
                        media_type
                    ),
                })
            }
        })
    }
}

impl TryFromLLM<openai::InputContent> for AssistantContentPart {
    type Error = ConvertError;

    fn try_from(value: openai::InputContent) -> Result<Self, Self::Error> {
        Ok(match value.input_content_type {
            openai::InputItemContentListType::InputText
            | openai::InputItemContentListType::OutputText => {
                // Build provider_options to preserve annotations and logprobs
                let provider_options = {
                    let mut options = serde_json::Map::new();
                    if let Some(annotations) = &value.annotations {
                        if !annotations.is_empty() {
                            options.insert(
                                "annotations".to_string(),
                                serde_json::to_value(annotations).map_err(|e| {
                                    ConvertError::JsonSerializationFailed {
                                        field: "annotations".to_string(),
                                        error: e.to_string(),
                                    }
                                })?,
                            );
                        }
                    }
                    if let Some(logprobs) = &value.logprobs {
                        if !logprobs.is_empty() {
                            options.insert(
                                "logprobs".to_string(),
                                serde_json::to_value(logprobs).map_err(|e| {
                                    ConvertError::JsonSerializationFailed {
                                        field: "logprobs".to_string(),
                                        error: e.to_string(),
                                    }
                                })?,
                            );
                        }
                    }
                    if options.is_empty() {
                        None
                    } else {
                        Some(crate::universal::message::ProviderOptions { options })
                    }
                };

                AssistantContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "text".to_string(),
                        })?,
                    encrypted_content: None,
                    cache_control: None,
                    provider_options,
                })
            }
            // TODO: ToolCall content type support - not yet implemented in generated types
            _ => {
                return Err(ConvertError::UnsupportedInputType {
                    type_info: format!("InputContent type: {:?}", value.input_content_type),
                });
            }
        })
    }
}

/// Default implementation for InputItem
impl Default for openai::InputItem {
    fn default() -> Self {
        Self {
            role: None,
            content: None,
            phase: None,
            input_item_type: None,
            status: None,
            id: None,
            queries: None,
            results: None,
            action: None,
            actions: None,
            call_id: None,
            pending_safety_checks: None,
            acknowledged_safety_checks: None,
            output: None,
            arguments: None,
            name: None,
            namespace: None,
            execution: None,
            encrypted_content: None,
            summary: None,
            result: None,
            code: None,
            fingerprint: None,
            caller: None,
            container_id: None,
            environment: None,
            max_output_length: None,
            operation: None,
            server_label: None,
            tools: None,
            approval_request_id: None,
            approve: None,
            reason: None,
            input: None,
            error: None,
            outputs: None,
            request_id: None,
        }
    }
}

/// Convert universal Message to OpenAI InputItem (for Responses API)
impl TryFromLLM<Message> for openai::InputItem {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::System { content } => Ok(openai::InputItem {
                role: Some(openai::InputItemRole::System),
                content: Some(TryFromLLM::try_from(content)?),
                input_item_type: Some(openai::InputItemType::Message),
                ..Default::default()
            }),
            Message::Developer { content } => Ok(openai::InputItem {
                role: Some(openai::InputItemRole::Developer),
                content: Some(TryFromLLM::try_from(content)?),
                input_item_type: Some(openai::InputItemType::Message),
                ..Default::default()
            }),
            Message::User { content } => Ok(openai::InputItem {
                role: Some(openai::InputItemRole::User),
                content: Some(TryFromLLM::try_from(content)?),
                input_item_type: Some(openai::InputItemType::Message),
                ..Default::default()
            }),
            Message::Assistant { content, id } => {
                match content {
                    AssistantContent::String(text) => Ok(openai::InputItem {
                        role: Some(openai::InputItemRole::Assistant),
                        content: Some(openai::InputItemContent::String(text)),
                        id,
                        input_item_type: Some(openai::InputItemType::Message),
                        status: Some(openai::FunctionCallItemStatus::Completed),
                        ..Default::default()
                    }),
                    AssistantContent::Array(parts) => {
                        let mut has_reasoning = false;
                        let mut encrypted_content = None;
                        let mut reasoning_parts: Vec<openai::SummaryText> = vec![];
                        let mut normal_parts: Vec<openai::InputContent> = vec![];
                        let mut tool_call_info: Option<ResponsesToolCallInfo> = None;
                        let mut discovery_call_info: Option<ResponsesDiscoveryCallInfo> = None;
                        let mut message_phase: Option<openai::MessagePhase> = None;

                        for part in parts {
                            match part {
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content: ec,
                                } => {
                                    has_reasoning = true;
                                    encrypted_content = ec;
                                    if !text.is_empty() {
                                        reasoning_parts.push(openai::SummaryText {
                                            text,
                                            summary_text_type: openai::SummaryType::SummaryText,
                                        });
                                    }
                                }
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    status,
                                    caller,
                                    encrypted_content: _,
                                    provider_options,
                                    provider_executed,
                                } => {
                                    tool_call_info = Some(ResponsesToolCallInfo {
                                        tool_call_id,
                                        tool_name,
                                        arguments: arguments.clone(),
                                        namespace: openai_tool_call_provider_options_view(
                                            &provider_options,
                                        )
                                        .and_then(|opts| opts.namespace),
                                        status,
                                        caller,
                                        provider_executed,
                                    });
                                }
                                AssistantContentPart::ToolDiscoveryCall {
                                    tool_call_id,
                                    discovery_tool_name: _,
                                    query,
                                    arguments,
                                    status,
                                    execution,
                                    ..
                                } => {
                                    discovery_call_info = Some(ResponsesDiscoveryCallInfo {
                                        tool_call_id,
                                        query,
                                        arguments,
                                        status,
                                        execution,
                                    });
                                }
                                other_part => {
                                    if let AssistantContentPart::Text(text_part) = &other_part {
                                        if let Some(phase) = openai_text_provider_options_view(
                                            &text_part.provider_options,
                                        )?
                                        .phase
                                        {
                                            if message_phase
                                                .as_ref()
                                                .is_some_and(|current| current != &phase)
                                            {
                                                return Err(ConvertError::UnsupportedMapping {
                                                    from: "conflicting Responses message phases"
                                                        .to_string(),
                                                    to: "OpenAI Responses input item",
                                                });
                                            }
                                            message_phase = Some(phase);
                                        }
                                    }
                                    normal_parts.push(TryFromLLM::try_from(other_part)?);
                                }
                            }
                        }

                        if has_reasoning {
                            if tool_call_info.is_some()
                                || discovery_call_info.is_some()
                                || !normal_parts.is_empty()
                            {
                                return Err(ConvertError::ContentConversionFailed {
                                    reason: "Mixed reasoning and other content parts are not supported in OpenAI format".to_string(),
                                });
                            }

                            // Pure reasoning message - convert to reasoning InputItem
                            let reasoning_item = openai::InputItem {
                                role: None, // Don't set role for reasoning items - let the original data determine this
                                content: Some(openai::InputItemContent::InputContentArray(vec![])),
                                input_item_type: Some(openai::InputItemType::Reasoning),
                                id: id.clone(),
                                summary: Some(reasoning_parts),
                                encrypted_content,
                                ..Default::default()
                            };
                            Ok(reasoning_item)
                        } else if let Some(discovery_call) = discovery_call_info {
                            if tool_call_info.is_some() || !normal_parts.is_empty() {
                                return Err(ConvertError::ContentConversionFailed {
                                    reason: "Mixed tool discovery and other content parts are not supported in OpenAI format".to_string(),
                                });
                            }
                            Ok(tool_discovery::input_call_from_universal(
                                discovery_call.tool_call_id,
                                discovery_call.query,
                                discovery_call.arguments,
                                discovery_call.status,
                                discovery_call.execution,
                                id.clone(),
                            ))
                        } else if let Some(tool_call) = tool_call_info {
                            if !normal_parts.is_empty() {
                                return Err(ConvertError::ContentConversionFailed {
                                    reason: "Mixed tool call and normal content parts are not supported in OpenAI format".to_string(),
                                });
                            }

                            // Check if this is a provider-executed built-in tool
                            if tool_call.provider_executed == Some(true) {
                                // Convert back to the appropriate built-in tool type based on tool_name
                                let args_value = match &tool_call.arguments {
                                    ToolCallArguments::Valid(map) => {
                                        serde_json::Value::Object(map.clone())
                                    }
                                    ToolCallArguments::Invalid(s)
                                    | ToolCallArguments::Custom(s) => {
                                        serde_json::Value::String(s.clone())
                                    }
                                };

                                let (input_item_type, mut item) = match &*tool_call.tool_name {
                                    "web_search" => (
                                        openai::InputItemType::WebSearchCall,
                                        openai::InputItem {
                                            action: parse_builtin_field(
                                                &args_value,
                                                "action",
                                                "web_search",
                                            )?,
                                            queries: parse_builtin_field(
                                                &args_value,
                                                "queries",
                                                "web_search",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "code_interpreter" => (
                                        openai::InputItemType::CodeInterpreterCall,
                                        openai::InputItem {
                                            code: args_value
                                                .get("code")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            container_id: args_value
                                                .get("container_id")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            outputs: parse_builtin_field(
                                                &args_value,
                                                "outputs",
                                                "code_interpreter",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "file_search" => (
                                        openai::InputItemType::FileSearchCall,
                                        openai::InputItem {
                                            queries: parse_builtin_field(
                                                &args_value,
                                                "queries",
                                                "file_search",
                                            )?,
                                            results: parse_builtin_field(
                                                &args_value,
                                                "results",
                                                "file_search",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "computer" => (
                                        openai::InputItemType::ComputerCall,
                                        openai::InputItem {
                                            action: parse_builtin_field(
                                                &args_value,
                                                "action",
                                                "computer",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "image_generation" => (
                                        openai::InputItemType::ImageGenerationCall,
                                        openai::InputItem {
                                            result: args_value
                                                .get("result")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            ..Default::default()
                                        },
                                    ),
                                    "local_shell" => (
                                        openai::InputItemType::LocalShellCall,
                                        openai::InputItem {
                                            action: parse_builtin_field(
                                                &args_value,
                                                "action",
                                                "local_shell",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "mcp_call" => (
                                        openai::InputItemType::McpCall,
                                        openai::InputItem {
                                            server_label: args_value
                                                .get("server_label")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            ..Default::default()
                                        },
                                    ),
                                    "mcp_list_tools" => (
                                        openai::InputItemType::McpListTools,
                                        openai::InputItem {
                                            server_label: args_value
                                                .get("server_label")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            tools: parse_builtin_field(
                                                &args_value,
                                                "tools",
                                                "mcp_list_tools",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "mcp_approval_request" => (
                                        openai::InputItemType::McpApprovalRequest,
                                        openai::InputItem {
                                            ..Default::default()
                                        },
                                    ),
                                    _ => {
                                        // Unknown provider-executed tool - fall back to FunctionCall
                                        return Ok(openai::InputItem {
                                            role: None,
                                            content: None,
                                            input_item_type: Some(
                                                openai::InputItemType::FunctionCall,
                                            ),
                                            id: id.clone(),
                                            call_id: Some(tool_call.tool_call_id),
                                            name: Some(tool_call.tool_name),
                                            namespace: tool_call.namespace,
                                            arguments: Some(openai_arguments_from_string(
                                                tool_call.arguments.to_string(),
                                            )),
                                            caller: tool_call.caller.map(Into::into),
                                            status: Some(openai::FunctionCallItemStatus::Completed),
                                            ..Default::default()
                                        });
                                    }
                                };

                                // Set common fields
                                item.id = id.clone();
                                item.input_item_type = Some(input_item_type);
                                item.status = args_value
                                    .get("status")
                                    .and_then(|v| serde_json::from_value(v.clone()).ok());

                                Ok(item)
                            } else {
                                let output_item_status = tool_call
                                    .status
                                    .as_deref()
                                    .map(|status| {
                                        function_call_item_status_from_string(
                                            status,
                                            "Responses function call status",
                                        )
                                    })
                                    .transpose()?
                                    .unwrap_or(openai::FunctionCallItemStatus::Completed);
                                let (input_item_type, input, arguments) = match &tool_call.arguments
                                {
                                    ToolCallArguments::Custom(input) => (
                                        openai::InputItemType::CustomToolCall,
                                        Some(input.clone()),
                                        None,
                                    ),
                                    arguments => (
                                        openai::InputItemType::FunctionCall,
                                        None,
                                        Some(openai_arguments_from_string(arguments.to_string())),
                                    ),
                                };
                                let tool_call_item = openai::InputItem {
                                    role: None, // Preserve original role state - request context function calls don't have roles
                                    content: None,
                                    input_item_type: Some(input_item_type),
                                    id: id.clone(),
                                    call_id: Some(tool_call.tool_call_id),
                                    name: Some(tool_call.tool_name),
                                    namespace: tool_call.namespace,
                                    input,
                                    arguments,
                                    caller: tool_call.caller.map(Into::into),
                                    status: Some(output_item_status),
                                    ..Default::default()
                                };
                                Ok(tool_call_item)
                            }
                        } else {
                            // Regular message - use normal conversion
                            Ok(openai::InputItem {
                                role: Some(openai::InputItemRole::Assistant),
                                content: Some(openai::InputItemContent::InputContentArray(
                                    normal_parts,
                                )),
                                input_item_type: Some(openai::InputItemType::Message),
                                id,
                                status: Some(openai::FunctionCallItemStatus::Completed),
                                phase: message_phase,
                                ..Default::default()
                            })
                        }
                    }
                }
            }
            Message::Tool { content } => {
                // Convert tool results to appropriate InputItems
                let mut result_items = Vec::new();

                for tool_part in content {
                    match tool_part {
                        ToolContentPart::ToolResult(tool_result) => {
                            // Convert tool result output to string for Refusal::String
                            let output_string = match &tool_result.output {
                                serde_json::Value::String(s) => s.clone(),
                                other => serde_json::to_string(other).map_err(|e| {
                                    ConvertError::JsonSerializationFailed {
                                        field: "tool_result_output".to_string(),
                                        error: e.to_string(),
                                    }
                                })?,
                            };

                            let input_item_type = if tool_result.custom_tool_call == Some(true) {
                                openai::InputItemType::CustomToolCallOutput
                            } else {
                                openai::InputItemType::FunctionCallOutput
                            };

                            result_items.push(openai::InputItem {
                                role: None,    // Function call outputs don't have roles
                                content: None, // Function call outputs use the output field, not content
                                input_item_type: Some(input_item_type),
                                call_id: Some(tool_result.tool_call_id.clone()),
                                output: Some(openai_output_from_string(output_string)),
                                caller: tool_result.caller.clone().map(Into::into),
                                name: if tool_result.tool_name.is_empty() {
                                    None
                                } else {
                                    Some(tool_result.tool_name.clone())
                                },
                                ..Default::default()
                            });
                        }
                        ToolContentPart::ToolDiscoveryResult(discovery_result) => {
                            result_items.push(tool_discovery::input_output_from_universal(
                                discovery_result,
                            )?);
                        }
                    }
                }

                // For now, return the first tool result or a placeholder
                result_items.into_iter().next().ok_or_else(|| {
                    ConvertError::ContentConversionFailed {
                        reason: "Empty tool content".to_string(),
                    }
                })
            }
            Message::AdditionalTools { tools, id } => Ok(openai::InputItem {
                role: Some(openai::InputItemRole::Developer),
                content: None,
                input_item_type: Some(openai::InputItemType::AdditionalTools),
                id,
                tools: Some(tool_discovery::input_item_tools_from_universal_tools(
                    &tools,
                )?),
                ..Default::default()
            }),
        }
    }
}

/// Create an InputItem for a function call (regular or built-in tool).
///
/// This helper extracts the logic for converting a universal tool call to an OpenAI InputItem,
/// handling both provider-executed built-in tools and regular function calls.
fn create_function_call_input_item(
    tool_call: &ResponsesToolCallInfo,
    id: Option<String>,
) -> Result<openai::InputItem, ConvertError> {
    let output_item_status = tool_call
        .status
        .as_deref()
        .map(|status| {
            function_call_item_status_from_string(status, "Responses function call status")
        })
        .transpose()?
        .unwrap_or(openai::FunctionCallItemStatus::Completed);

    // Check if this is a provider-executed built-in tool
    if tool_call.provider_executed == Some(true) {
        // Convert back to the appropriate built-in tool type based on tool_name
        let args_value = match &tool_call.arguments {
            ToolCallArguments::Valid(map) => serde_json::Value::Object(map.clone()),
            ToolCallArguments::Invalid(s) | ToolCallArguments::Custom(s) => {
                serde_json::Value::String(s.clone())
            }
        };

        let (input_item_type, mut item) = match &tool_call.tool_name[..] {
            "web_search" => (
                openai::InputItemType::WebSearchCall,
                openai::InputItem {
                    action: args_value
                        .get("action")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    queries: args_value
                        .get("queries")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "code_interpreter" => (
                openai::InputItemType::CodeInterpreterCall,
                openai::InputItem {
                    code: args_value
                        .get("code")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    container_id: args_value
                        .get("container_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    outputs: args_value
                        .get("outputs")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "file_search" => (
                openai::InputItemType::FileSearchCall,
                openai::InputItem {
                    queries: args_value
                        .get("queries")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    results: args_value
                        .get("results")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "computer" => (
                openai::InputItemType::ComputerCall,
                openai::InputItem {
                    action: args_value
                        .get("action")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "image_generation" => (
                openai::InputItemType::ImageGenerationCall,
                openai::InputItem {
                    result: args_value
                        .get("result")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ..Default::default()
                },
            ),
            "local_shell" => (
                openai::InputItemType::LocalShellCall,
                openai::InputItem {
                    action: args_value
                        .get("action")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "mcp_call" => (
                openai::InputItemType::McpCall,
                openai::InputItem {
                    server_label: args_value
                        .get("server_label")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ..Default::default()
                },
            ),
            "mcp_list_tools" => (
                openai::InputItemType::McpListTools,
                openai::InputItem {
                    server_label: args_value
                        .get("server_label")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    tools: args_value
                        .get("tools")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "mcp_approval_request" => (
                openai::InputItemType::McpApprovalRequest,
                openai::InputItem {
                    ..Default::default()
                },
            ),
            _ => {
                // Unknown provider-executed tool - fall back to FunctionCall
                return Ok(openai::InputItem {
                    role: None,
                    content: None,
                    input_item_type: Some(openai::InputItemType::FunctionCall),
                    id,
                    call_id: Some(tool_call.tool_call_id.clone()),
                    name: Some(tool_call.tool_name.clone()),
                    namespace: tool_call.namespace.clone(),
                    caller: tool_call.caller.clone().map(Into::into),
                    arguments: Some(openai_arguments_from_string(
                        tool_call.arguments.to_string(),
                    )),
                    status: Some(output_item_status),
                    ..Default::default()
                });
            }
        };

        // Set common fields
        item.id = id;
        item.input_item_type = Some(input_item_type);
        item.caller = tool_call.caller.clone().map(Into::into);
        item.status = args_value
            .get("status")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        Ok(item)
    } else if let ToolCallArguments::Custom(input) = &tool_call.arguments {
        Ok(openai::InputItem {
            role: None,
            content: None,
            input_item_type: Some(openai::InputItemType::CustomToolCall),
            id,
            call_id: Some(tool_call.tool_call_id.clone()),
            name: Some(tool_call.tool_name.clone()),
            namespace: tool_call.namespace.clone(),
            caller: tool_call.caller.clone().map(Into::into),
            input: Some(input.clone()),
            status: Some(output_item_status),
            ..Default::default()
        })
    } else {
        Ok(openai::InputItem {
            role: None, // Preserve original role state - request context function calls don't have roles
            content: None,
            input_item_type: Some(openai::InputItemType::FunctionCall),
            id,
            call_id: Some(tool_call.tool_call_id.clone()),
            name: Some(tool_call.tool_name.clone()),
            namespace: tool_call.namespace.clone(),
            caller: tool_call.caller.clone().map(Into::into),
            arguments: Some(openai_arguments_from_string(
                tool_call.arguments.to_string(),
            )),
            status: Some(output_item_status),
            ..Default::default()
        })
    }
}

/// Convert universal messages to OpenAI Responses API InputItem format.
///
/// This function handles the 1:N expansion for Tool messages - a single Tool message
/// can contain multiple tool results, and each result becomes a separate InputItem
/// (which is required by the Responses API).
///
/// It also handles 1:N expansion for Assistant messages with mixed content (reasoning,
/// text, and tool calls). Each content type becomes a separate InputItem in order:
/// 1. Reasoning item (if reasoning parts exist)
/// 2. Message item (if text/normal parts exist)
/// 3. Function call items (one per tool call)
///
/// This is provided as a standalone function rather than a TryFromLLM impl because
/// Rust's coherence rules don't allow overriding the blanket Vec implementation.
pub fn universal_to_responses_input(
    messages: &[Message],
) -> Result<Vec<openai::InputItem>, ConvertError> {
    let mut result = Vec::with_capacity(messages.len());

    for msg in messages {
        match msg {
            Message::Tool { content } => {
                // Expand: one Tool message → multiple InputItems
                for tool_part in content {
                    match tool_part {
                        ToolContentPart::ToolResult(tool_result) => {
                            let output_string = match &tool_result.output {
                                serde_json::Value::String(s) => s.clone(),
                                other => serde_json::to_string(other).map_err(|e| {
                                    ConvertError::JsonSerializationFailed {
                                        field: "tool_result_output".to_string(),
                                        error: e.to_string(),
                                    }
                                })?,
                            };

                            let input_item_type = if tool_result.custom_tool_call == Some(true) {
                                openai::InputItemType::CustomToolCallOutput
                            } else {
                                openai::InputItemType::FunctionCallOutput
                            };

                            result.push(openai::InputItem {
                                role: None,
                                content: None,
                                input_item_type: Some(input_item_type),
                                call_id: Some(tool_result.tool_call_id.clone()),
                                output: Some(openai_output_from_string(output_string)),
                                caller: tool_result.caller.clone().map(Into::into),
                                name: if tool_result.tool_name.is_empty() {
                                    None
                                } else {
                                    Some(tool_result.tool_name.clone())
                                },
                                ..Default::default()
                            });
                        }
                        ToolContentPart::ToolDiscoveryResult(discovery_result) => {
                            result.push(tool_discovery::input_output_from_universal(
                                discovery_result.clone(),
                            )?);
                        }
                    }
                }
            }
            Message::Assistant { content, id } => {
                // Handle assistant messages with potential 1:N expansion for mixed content
                match content {
                    AssistantContent::String(text) => {
                        // Simple case: single message item
                        result.push(openai::InputItem {
                            role: Some(openai::InputItemRole::Assistant),
                            content: Some(openai::InputItemContent::String(text.clone())),
                            id: id.clone(),
                            input_item_type: Some(openai::InputItemType::Message),
                            status: Some(openai::FunctionCallItemStatus::Completed),
                            ..Default::default()
                        });
                    }
                    AssistantContent::Array(parts) => {
                        // Categorize all parts into separate collections
                        let mut reasoning_parts: Vec<openai::SummaryText> = vec![];
                        let mut has_reasoning = false;
                        let mut encrypted_content = None;
                        let mut normal_parts: Vec<openai::InputContent> = vec![];
                        let mut message_phase: Option<openai::MessagePhase> = None;
                        let mut sequenced_items: Vec<ResponsesSequencedInputItem> = vec![];

                        for part in parts {
                            match part {
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content: ec,
                                } => {
                                    has_reasoning = true;
                                    encrypted_content = ec.clone();
                                    if !text.is_empty() {
                                        reasoning_parts.push(openai::SummaryText {
                                            text: text.clone(),
                                            summary_text_type: openai::SummaryType::SummaryText,
                                        });
                                    }
                                }
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    status,
                                    caller,
                                    encrypted_content: _,
                                    provider_options,
                                    provider_executed,
                                } => {
                                    sequenced_items.push(ResponsesSequencedInputItem::ToolCall(
                                        ResponsesToolCallInfo {
                                            tool_call_id: tool_call_id.clone(),
                                            tool_name: tool_name.clone(),
                                            arguments: arguments.clone(),
                                            namespace: openai_tool_call_provider_options_view(
                                                provider_options,
                                            )
                                            .and_then(|opts| opts.namespace),
                                            status: status.clone(),
                                            caller: caller.clone(),
                                            provider_executed: *provider_executed,
                                        },
                                    ));
                                }
                                AssistantContentPart::ToolDiscoveryCall {
                                    tool_call_id,
                                    discovery_tool_name: _,
                                    query,
                                    arguments,
                                    status,
                                    execution,
                                    ..
                                } => {
                                    sequenced_items.push(
                                        ResponsesSequencedInputItem::DiscoveryCall(
                                            ResponsesDiscoveryCallInfo {
                                                tool_call_id: tool_call_id.clone(),
                                                query: query.clone(),
                                                arguments: arguments.clone(),
                                                status: status.clone(),
                                                execution: execution.clone(),
                                            },
                                        ),
                                    );
                                }
                                AssistantContentPart::Program {
                                    call_id,
                                    code,
                                    fingerprint,
                                    id,
                                } => {
                                    sequenced_items.push(ResponsesSequencedInputItem::Item(
                                        Box::new(openai::InputItem {
                                            input_item_type: Some(openai::InputItemType::Program),
                                            id: id.clone(),
                                            call_id: Some(call_id.clone()),
                                            code: Some(code.clone()),
                                            fingerprint: fingerprint.clone(),
                                            ..Default::default()
                                        }),
                                    ));
                                }
                                AssistantContentPart::ProgramOutput {
                                    call_id,
                                    result,
                                    status,
                                    id,
                                } => {
                                    sequenced_items.push(ResponsesSequencedInputItem::Item(
                                        Box::new(openai::InputItem {
                                            input_item_type: Some(
                                                openai::InputItemType::ProgramOutput,
                                            ),
                                            id: id.clone(),
                                            call_id: Some(call_id.clone()),
                                            result: Some(result.clone()),
                                            status: Some(
                                                serde_json::from_value(serde_json::Value::String(
                                                    status.clone(),
                                                ))
                                                .map_err(|e| ConvertError::InvalidEnumValue {
                                                    type_name: "FunctionCallItemStatus",
                                                    value: e.to_string(),
                                                })?,
                                            ),
                                            ..Default::default()
                                        }),
                                    ));
                                }
                                other_part => {
                                    if let AssistantContentPart::Text(text_part) = other_part {
                                        if let Some(phase) = openai_text_provider_options_view(
                                            &text_part.provider_options,
                                        )?
                                        .phase
                                        {
                                            if message_phase
                                                .as_ref()
                                                .is_some_and(|current| current != &phase)
                                            {
                                                return Err(ConvertError::UnsupportedMapping {
                                                    from: "conflicting Responses message phases"
                                                        .to_string(),
                                                    to: "OpenAI Responses input item",
                                                });
                                            }
                                            message_phase = Some(phase);
                                        }
                                    }
                                    normal_parts.push(TryFromLLM::try_from(other_part.clone())?);
                                }
                            }
                        }

                        // 1. Emit reasoning item if any reasoning part existed (even with empty text)
                        if has_reasoning {
                            result.push(openai::InputItem {
                                role: None,
                                content: Some(openai::InputItemContent::InputContentArray(vec![])),
                                input_item_type: Some(openai::InputItemType::Reasoning),
                                id: id.clone(),
                                summary: Some(reasoning_parts),
                                encrypted_content: encrypted_content.clone(),
                                ..Default::default()
                            });
                        }

                        // 2. Emit message item if normal parts present
                        if !normal_parts.is_empty() {
                            result.push(openai::InputItem {
                                role: Some(openai::InputItemRole::Assistant),
                                content: Some(openai::InputItemContent::InputContentArray(
                                    normal_parts,
                                )),
                                input_item_type: Some(openai::InputItemType::Message),
                                // Only clear id if reasoning was emitted (it used the id)
                                id: if has_reasoning { None } else { id.clone() },
                                status: Some(openai::FunctionCallItemStatus::Completed),
                                phase: message_phase,
                                ..Default::default()
                            });
                        }

                        // 3. Emit tool/program items in their original relative order.
                        for sequenced_item in sequenced_items {
                            match sequenced_item {
                                ResponsesSequencedInputItem::ToolCall(tool_call) => {
                                    result.push(create_function_call_input_item(
                                        &tool_call,
                                        id.clone(),
                                    )?);
                                }
                                ResponsesSequencedInputItem::DiscoveryCall(discovery_call) => {
                                    result.push(tool_discovery::input_call_from_universal(
                                        discovery_call.tool_call_id,
                                        discovery_call.query,
                                        discovery_call.arguments,
                                        discovery_call.status,
                                        discovery_call.execution,
                                        id.clone(),
                                    ));
                                }
                                ResponsesSequencedInputItem::Item(item) => result.push(*item),
                            }
                        }
                    }
                }
            }
            Message::AdditionalTools { tools, id } => {
                result.push(openai::InputItem {
                    role: Some(openai::InputItemRole::Developer),
                    content: None,
                    input_item_type: Some(openai::InputItemType::AdditionalTools),
                    id: id.clone(),
                    tools: Some(tool_discovery::input_item_tools_from_universal_tools(
                        tools,
                    )?),
                    ..Default::default()
                });
            }
            other => {
                // For all other message types, use the standard conversion
                result.push(<openai::InputItem as TryFromLLM<Message>>::try_from(
                    other.clone(),
                )?);
            }
        }
    }

    Ok(result)
}

/// Convert OutputItem to InputItem for unified processing
/// OutputItem is used in responses while InputItem is used in requests,
/// but they have very similar structure for message content
impl TryFromLLM<openai::OutputItem> for openai::InputItem {
    type Error = ConvertError;

    fn try_from(output_item: openai::OutputItem) -> Result<Self, Self::Error> {
        // Convert OutputItem to InputItem by mapping the fields
        // The main differences are in content type and some field names

        let input_item_type = match output_item.output_item_type {
            Some(openai::OutputItemType::Message) => Some(openai::InputItemType::Message),
            Some(openai::OutputItemType::Reasoning) => Some(openai::InputItemType::Reasoning),
            Some(openai::OutputItemType::FunctionCall) => Some(openai::InputItemType::FunctionCall),
            Some(openai::OutputItemType::CustomToolCall) => {
                Some(openai::InputItemType::CustomToolCall)
            }
            Some(openai::OutputItemType::Program) => Some(openai::InputItemType::Program),
            Some(openai::OutputItemType::ProgramOutput) => {
                Some(openai::InputItemType::ProgramOutput)
            }
            // Map built-in tool types for proper handling during conversion
            Some(openai::OutputItemType::CodeInterpreterCall) => {
                Some(openai::InputItemType::CodeInterpreterCall)
            }
            Some(openai::OutputItemType::WebSearchCall) => {
                Some(openai::InputItemType::WebSearchCall)
            }
            Some(openai::OutputItemType::FileSearchCall) => {
                Some(openai::InputItemType::FileSearchCall)
            }
            Some(openai::OutputItemType::ComputerCall) => Some(openai::InputItemType::ComputerCall),
            Some(openai::OutputItemType::ImageGenerationCall) => {
                Some(openai::InputItemType::ImageGenerationCall)
            }
            Some(openai::OutputItemType::LocalShellCall) => {
                Some(openai::InputItemType::LocalShellCall)
            }
            Some(openai::OutputItemType::McpCall) => Some(openai::InputItemType::McpCall),
            Some(openai::OutputItemType::McpListTools) => Some(openai::InputItemType::McpListTools),
            Some(openai::OutputItemType::McpApprovalRequest) => {
                Some(openai::InputItemType::McpApprovalRequest)
            }
            Some(openai::OutputItemType::ToolSearchCall) => {
                Some(openai::InputItemType::ToolSearchCall)
            }
            Some(openai::OutputItemType::ToolSearchOutput) => {
                Some(openai::InputItemType::ToolSearchOutput)
            }
            Some(openai::OutputItemType::AdditionalTools) => {
                Some(openai::InputItemType::AdditionalTools)
            }
            // For other types, we might need to map them or handle specially
            _ => None, // Will be handled based on content
        };

        // Convert content from Vec<OutputMessageContent> to InputItemContent
        let content = if let Some(output_content) = output_item.content {
            if output_content.is_empty() {
                None
            } else if output_content.len() == 1 {
                // Single content item - check if we can convert to string
                // Only convert to string if there are no annotations or other metadata to preserve
                let first = &output_content[0];
                let has_annotations = first
                    .annotations
                    .as_ref()
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                let has_logprobs = first
                    .logprobs
                    .as_ref()
                    .map(|l| !l.is_empty())
                    .unwrap_or(false);

                if first.output_message_content_type == openai::ContentType::OutputText
                    && !has_annotations
                    && !has_logprobs
                {
                    output_content
                        .into_iter()
                        .next()
                        .unwrap()
                        .text
                        .map(openai::InputItemContent::String)
                } else {
                    // Convert to InputContent array to preserve annotations/logprobs
                    let input_contents: Result<Vec<_>, _> = output_content
                        .into_iter()
                        .map(convert_output_message_content_to_input_content)
                        .collect();
                    Some(openai::InputItemContent::InputContentArray(input_contents?))
                }
            } else {
                // Multiple content items - convert to array
                let input_contents: Result<Vec<_>, _> = output_content
                    .into_iter()
                    .map(convert_output_message_content_to_input_content)
                    .collect();
                Some(openai::InputItemContent::InputContentArray(input_contents?))
            }
        } else {
            None
        };

        // Convert role from RoleEnum to InputItemRole
        // If no role is provided, infer it from the output_item_type only for certain types
        let role = output_item
            .role
            .map(|mr| match mr {
                openai::RoleEnum::Assistant => Ok(openai::InputItemRole::Assistant),
                openai::RoleEnum::Developer => Ok(openai::InputItemRole::Developer),
                openai::RoleEnum::System => Ok(openai::InputItemRole::System),
                openai::RoleEnum::User => Ok(openai::InputItemRole::User),
                other => Err(ConvertError::UnsupportedMapping {
                    from: format!("RoleEnum::{:?}", other),
                    to: "InputItemRole",
                }),
            })
            .transpose()?
            .or({
                // Only infer role for regular messages, not for function calls or other items
                // Function calls and other tool-related items should preserve their original role state
                match output_item.output_item_type {
                    Some(openai::OutputItemType::Message) => Some(openai::InputItemRole::Assistant),
                    _ => None, // Don't infer role for function calls, reasoning, and other types
                }
            });

        // Convert status
        let status = output_item.status;

        // Handle reasoning summary conversion - OutputItem has summary field
        let summary = output_item.summary;

        Ok(openai::InputItem {
            role,
            content,
            input_item_type,
            status,
            id: output_item.id,
            summary,
            // Preserve structured function call fields
            arguments: output_item_arguments_to_input(output_item.arguments),
            name: output_item.name,
            namespace: output_item.namespace,
            // Set other fields to None/default - many OutputItem fields don't have InputItem equivalents
            queries: output_item.queries,
            call_id: output_item.call_id,
            results: output_item.results,
            action: output_item.action.and_then(|action| {
                serde_json::to_value(action)
                    .and_then(serde_json::from_value)
                    .ok()
            }),
            pending_safety_checks: output_item.pending_safety_checks,
            acknowledged_safety_checks: None,
            output: output_item_output_to_input(output_item.output),
            encrypted_content: output_item.encrypted_content,
            result: output_item.result,
            code: output_item.code,
            fingerprint: output_item.fingerprint,
            caller: output_item.caller.map(ToolCaller::from).map(Into::into),
            container_id: output_item.container_id,
            outputs: output_item.outputs,
            execution: output_item.execution,
            error: output_item.error,
            server_label: output_item.server_label,
            tools: output_item.tools.and_then(|tools| {
                serde_json::to_value(tools)
                    .and_then(serde_json::from_value)
                    .ok()
            }),
            approval_request_id: None,
            approve: None,
            reason: None,
            input: output_item.input,
            request_id: output_item.request_id,
            ..Default::default()
        })
    }
}

/// Convert InputItem to OutputItem (reverse of OutputItem -> InputItem conversion)
impl TryFromLLM<openai::InputItem> for openai::OutputItem {
    type Error = ConvertError;

    fn try_from(input_item: openai::InputItem) -> Result<Self, Self::Error> {
        // Convert InputItem to OutputItem by mapping the fields
        let output_item_type = match input_item.input_item_type {
            Some(openai::InputItemType::Message) => Some(openai::OutputItemType::Message),
            Some(openai::InputItemType::Reasoning) => Some(openai::OutputItemType::Reasoning),
            Some(openai::InputItemType::FunctionCall) => Some(openai::OutputItemType::FunctionCall),
            Some(openai::InputItemType::CustomToolCall) => {
                Some(openai::OutputItemType::CustomToolCall)
            }
            Some(openai::InputItemType::Program) => Some(openai::OutputItemType::Program),
            Some(openai::InputItemType::ProgramOutput) => {
                Some(openai::OutputItemType::ProgramOutput)
            }
            // Built-in tool types
            Some(openai::InputItemType::CodeInterpreterCall) => {
                Some(openai::OutputItemType::CodeInterpreterCall)
            }
            Some(openai::InputItemType::WebSearchCall) => {
                Some(openai::OutputItemType::WebSearchCall)
            }
            Some(openai::InputItemType::FileSearchCall) => {
                Some(openai::OutputItemType::FileSearchCall)
            }
            Some(openai::InputItemType::ComputerCall) => Some(openai::OutputItemType::ComputerCall),
            Some(openai::InputItemType::ImageGenerationCall) => {
                Some(openai::OutputItemType::ImageGenerationCall)
            }
            Some(openai::InputItemType::LocalShellCall) => {
                Some(openai::OutputItemType::LocalShellCall)
            }
            Some(openai::InputItemType::McpCall) => Some(openai::OutputItemType::McpCall),
            Some(openai::InputItemType::McpListTools) => Some(openai::OutputItemType::McpListTools),
            Some(openai::InputItemType::McpApprovalRequest) => {
                Some(openai::OutputItemType::McpApprovalRequest)
            }
            Some(openai::InputItemType::ToolSearchCall) => {
                Some(openai::OutputItemType::ToolSearchCall)
            }
            Some(openai::InputItemType::ToolSearchOutput) => {
                Some(openai::OutputItemType::ToolSearchOutput)
            }
            Some(openai::InputItemType::AdditionalTools) => {
                Some(openai::OutputItemType::AdditionalTools)
            }
            _ => None,
        };

        // Convert content from InputItemContent to Vec<OutputMessageContent>
        let content = if let Some(input_content) = input_item.content {
            match input_content {
                openai::InputItemContent::String(text) => {
                    // Single string content becomes single OutputMessageContent
                    Some(vec![openai::OutputMessageContent {
                        output_message_content_type: openai::ContentType::OutputText,
                        text: Some(text),
                        annotations: Some(vec![]),
                        logprobs: Some(vec![]),
                        refusal: None,
                    }])
                }
                openai::InputItemContent::InputContentArray(input_contents) => {
                    // Convert InputContent array to OutputMessageContent array
                    let output_contents: Result<Vec<_>, _> = input_contents
                        .into_iter()
                        .map(convert_input_content_to_output_message_content)
                        .collect();
                    Some(output_contents?)
                }
            }
        } else {
            None
        };

        // Convert role from InputItemRole to RoleEnum
        let role = input_item.role.map(|ir| match ir {
            openai::InputItemRole::Assistant => openai::RoleEnum::Assistant,
            openai::InputItemRole::Developer => openai::RoleEnum::Developer,
            openai::InputItemRole::System => openai::RoleEnum::System,
            openai::InputItemRole::User => openai::RoleEnum::User,
        });

        Ok(openai::OutputItem {
            role,
            content,
            output_item_type,
            status: input_item.status,
            id: input_item.id,
            summary: input_item.summary,
            arguments: input_item_arguments_to_output(input_item.arguments),
            name: input_item.name,
            namespace: input_item.namespace,
            queries: input_item.queries,
            call_id: input_item.call_id,
            results: input_item.results,
            action: input_item.action.and_then(|action| {
                serde_json::to_value(action)
                    .and_then(serde_json::from_value)
                    .ok()
            }),
            pending_safety_checks: input_item.pending_safety_checks,
            encrypted_content: input_item.encrypted_content,
            result: input_item.result,
            code: input_item.code,
            fingerprint: input_item.fingerprint,
            caller: input_item.caller.map(ToolCaller::from).map(Into::into),
            container_id: input_item.container_id,
            outputs: input_item.outputs,
            execution: input_item.execution,
            error: input_item.error,
            output: input_item_output_to_output(input_item.output),
            server_label: input_item.server_label,
            tools: input_item.tools.and_then(|tools| {
                serde_json::to_value(tools)
                    .and_then(serde_json::from_value)
                    .ok()
            }),
            input: input_item.input,
            request_id: input_item.request_id,
            ..Default::default()
        })
    }
}

/// Convert OpenAI OutputItem collection to universal Message collection.
/// Each OutputItem becomes a separate Message to preserve the structure.
impl TryFromLLM<Vec<openai::OutputItem>> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(items: Vec<openai::OutputItem>) -> Result<Vec<Message>, Self::Error> {
        let mut messages: Vec<Message> = Vec::new();
        let mut last_tool_search_call_id: Option<String> = None;

        for mut item in items {
            let item_id = item.id.clone();
            let is_tool_search_call = matches!(
                item.output_item_type,
                Some(openai::OutputItemType::ToolSearchCall)
            );
            let is_tool_search_output = matches!(
                item.output_item_type,
                Some(openai::OutputItemType::ToolSearchOutput)
            );

            if is_tool_search_output {
                if item.call_id.is_none() {
                    item.call_id = last_tool_search_call_id.clone();
                }
                last_tool_search_call_id = None;
                messages.push(tool_discovery::message_from_output_output(item)?);
                continue;
            }

            if !is_tool_search_call {
                last_tool_search_call_id = None;
            }

            let parts: Vec<AssistantContentPart> = match item.output_item_type {
                Some(openai::OutputItemType::Message) => {
                    // Extract text content from message output items.
                    // `phase` is a message-level field (not content-level); it is stored in
                    // the first text part's provider_options under the typed `phase` field so
                    // it survives the OutputItem → Message → OutputItem roundtrip.
                    let item_phase = item.phase.take();
                    let mut text_parts = Vec::new();
                    let mut phase_stored = false;
                    if let Some(content) = item.content {
                        for c in content {
                            if let Some(text) = c.text {
                                // Preserve logprobs and non-empty annotations in
                                // provider_options for round-trip fidelity.
                                // Empty `annotations` arrays are the Responses API default
                                // injected by `response_from_universal`, so they are not
                                // stored back to keep the universal representation clean.
                                let non_empty_annotations = c.annotations.filter(|a| !a.is_empty());
                                let mut options = serde_json::Map::new();
                                if let Some(annotations) = non_empty_annotations {
                                    if let Ok(value) = serde_json::to_value(&annotations) {
                                        options.insert("annotations".to_string(), value);
                                    }
                                }
                                if let Some(logprobs) = c.logprobs {
                                    if let Ok(value) = serde_json::to_value(&logprobs) {
                                        options.insert("logprobs".to_string(), value);
                                    }
                                }
                                if !phase_stored {
                                    phase_stored = true;
                                    if let Some(ref phase) = item_phase {
                                        if let Ok(value) = serde_json::to_value(phase) {
                                            options.insert("phase".to_string(), value);
                                        }
                                    }
                                }
                                let provider_options = if options.is_empty() {
                                    None
                                } else {
                                    Some(ProviderOptions { options })
                                };
                                text_parts.push(AssistantContentPart::Text(TextContentPart {
                                    text,
                                    encrypted_content: None,
                                    cache_control: None,
                                    provider_options,
                                }));
                            }
                        }
                    }
                    text_parts
                }
                Some(openai::OutputItemType::Reasoning) => {
                    // Convert reasoning output to reasoning content parts
                    let mut reasoning_parts = Vec::new();
                    let mut first = true;
                    for summary in item.summary.unwrap_or_default() {
                        reasoning_parts.push(AssistantContentPart::Reasoning {
                            text: summary.text,
                            encrypted_content: if first {
                                first = false;
                                item.encrypted_content.take()
                            } else {
                                None
                            },
                        });
                    }
                    // Handle empty reasoning (still preserve encrypted content)
                    if first {
                        reasoning_parts.push(AssistantContentPart::Reasoning {
                            text: String::new(),
                            encrypted_content: item.encrypted_content.take(),
                        });
                    }
                    reasoning_parts
                }
                Some(openai::OutputItemType::Program) => {
                    vec![AssistantContentPart::Program {
                        id: item.id.clone(),
                        call_id: item.call_id.ok_or_else(|| {
                            ConvertError::MissingRequiredField {
                                field: "program call_id".to_string(),
                            }
                        })?,
                        code: item
                            .code
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "program code".to_string(),
                            })?,
                        fingerprint: item.fingerprint,
                    }]
                }
                Some(openai::OutputItemType::ProgramOutput) => {
                    vec![AssistantContentPart::ProgramOutput {
                        id: item.id.clone(),
                        call_id: item.call_id.ok_or_else(|| {
                            ConvertError::MissingRequiredField {
                                field: "program_output call_id".to_string(),
                            }
                        })?,
                        result: item
                            .result
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "program_output result".to_string(),
                            })?,
                        status: item
                            .status
                            .map(|status| {
                                function_call_item_status_to_string(status, "program_output status")
                            })
                            .transpose()?
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "program_output status".to_string(),
                            })?,
                    }]
                }
                item_type @ (Some(openai::OutputItemType::FunctionCall)
                | Some(openai::OutputItemType::CustomToolCall)) => {
                    let tool_call_id =
                        item.call_id
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "tool call call_id".to_string(),
                            })?;
                    let tool_name =
                        item.name
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "tool call name".to_string(),
                            })?;
                    let arguments = match item_type {
                        Some(openai::OutputItemType::CustomToolCall) => {
                            ToolCallArguments::Custom(item.input.ok_or_else(|| {
                                ConvertError::MissingRequiredField {
                                    field: "custom tool call input".to_string(),
                                }
                            })?)
                        }
                        _ => {
                            let arguments_str = item
                                .arguments
                                .map(|value| match value {
                                    serde_json::Value::String(value) => value,
                                    value => value.to_string(),
                                })
                                .unwrap_or_else(|| EMPTY_OBJECT_STR.to_string());
                            arguments_str.into()
                        }
                    };

                    vec![AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments,
                        encrypted_content: None,
                        provider_options: provider_options_from_openai_tool_call(item.namespace),
                        status: non_completed_function_call_status_to_string(
                            item.status,
                            "Responses output function call status",
                        )?,
                        caller: item.caller.map(Into::into),
                        provider_executed: None,
                    }]
                }
                Some(openai::OutputItemType::ToolSearchCall) => {
                    if item.call_id.is_none() {
                        item.call_id = item.id.clone();
                    }
                    last_tool_search_call_id = item.call_id.clone();
                    vec![tool_discovery::part_from_output_call(item)?]
                }
                Some(openai::OutputItemType::CodeInterpreterCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "code_interpreter".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "code": item.code,
                            "container_id": item.container_id,
                            "outputs": item.outputs,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::WebSearchCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "web_search".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": item.action,
                            "queries": item.queries,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::FileSearchCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "file_search".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "queries": item.queries,
                            "results": item.results,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::ComputerCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "computer".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": item.action,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::ImageGenerationCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "image_generation".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "result": item.result,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::LocalShellCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "local_shell".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": item.action,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::McpCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "mcp_call".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "server_label": item.server_label,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::McpListTools) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "mcp_list_tools".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "server_label": item.server_label,
                            "tools": item.tools,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::McpApprovalRequest) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "mcp_approval_request".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::AdditionalTools) => {
                    messages.push(tool_discovery::message_from_output_additional_tools(item)?);
                    continue;
                }
                _ => {
                    // Skip unknown output item types
                    continue;
                }
            };

            // Only create a message if there are parts
            if !parts.is_empty() {
                messages.push(Message::Assistant {
                    content: AssistantContent::Array(parts),
                    id: item_id,
                });
            }
        }

        Ok(messages)
    }
}

/// Convert universal Message collection to OpenAI OutputItem collection
/// This leverages the Message -> InputItem -> OutputItem conversion chain
/// Convert universal Message collection to OpenAI OutputItem collection.
/// This directly converts content parts to OutputItems, preserving order.
impl TryFromLLM<Vec<Message>> for Vec<openai::OutputItem> {
    type Error = ConvertError;

    fn try_from(messages: Vec<Message>) -> Result<Self, Self::Error> {
        let mut result = Vec::new();

        for msg in messages {
            match msg {
                Message::Tool { content } => {
                    for part in content {
                        match part {
                            ToolContentPart::ToolDiscoveryResult(discovery_result) => {
                                result.push(tool_discovery::output_output_from_universal(
                                    discovery_result,
                                )?);
                            }
                            ToolContentPart::ToolResult(tool_result) => {
                                let output_string = match tool_result.output {
                                    serde_json::Value::String(s) => s,
                                    other => serde_json::to_string(&other).map_err(|e| {
                                        ConvertError::JsonSerializationFailed {
                                            field: "tool_result_output".to_string(),
                                            error: e.to_string(),
                                        }
                                    })?,
                                };
                                let output_item_type = if tool_result.custom_tool_call == Some(true)
                                {
                                    openai::OutputItemType::CustomToolCallOutput
                                } else {
                                    openai::OutputItemType::FunctionCallOutput
                                };

                                result.push(openai::OutputItem {
                                    output_item_type: Some(output_item_type),
                                    call_id: Some(tool_result.tool_call_id),
                                    name: (!tool_result.tool_name.is_empty())
                                        .then_some(tool_result.tool_name),
                                    caller: tool_result.caller.map(Into::into),
                                    output: input_item_output_to_output(Some(
                                        openai_output_from_string(output_string),
                                    )),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
                Message::AdditionalTools { tools, id } => {
                    let input_tools =
                        tool_discovery::input_item_tools_from_universal_tools(&tools)?;
                    let output_tools = serde_json::to_value(input_tools)
                        .and_then(serde_json::from_value)
                        .map_err(|e| ConvertError::JsonSerializationFailed {
                            field: "Responses additional_tools output tools".to_string(),
                            error: e.to_string(),
                        })?;
                    result.push(openai::OutputItem {
                        output_item_type: Some(openai::OutputItemType::AdditionalTools),
                        role: Some(openai::RoleEnum::Developer),
                        id,
                        tools: Some(output_tools),
                        ..Default::default()
                    });
                }
                Message::Assistant { content, id } => {
                    match content {
                        AssistantContent::String(text) => {
                            result.push(openai::OutputItem {
                                output_item_type: Some(openai::OutputItemType::Message),
                                role: Some(openai::RoleEnum::Assistant),
                                content: Some(vec![openai::OutputMessageContent {
                                    output_message_content_type: openai::ContentType::OutputText,
                                    text: Some(text),
                                    annotations: Some(vec![]),
                                    logprobs: None,
                                    refusal: None,
                                }]),
                                id,
                                status: Some(openai::FunctionCallItemStatus::Completed),
                                ..Default::default()
                            });
                        }
                        AssistantContent::Array(parts) => {
                            // Track whether we've assigned the id to prevent duplicate IDs
                            let mut id_used = false;
                            let use_id = |used: &mut bool, id: &Option<String>| -> Option<String> {
                                if *used {
                                    None
                                } else {
                                    *used = true;
                                    id.clone()
                                }
                            };

                            // Collect consecutive reasoning parts into a single OutputItem
                            let mut pending_reasoning_summaries: Vec<openai::SummaryText> = vec![];
                            let mut pending_encrypted_content: Option<String> = None;
                            let mut has_pending_reasoning = false;

                            let flush_reasoning =
                                |result: &mut Vec<openai::OutputItem>,
                                 summaries: &mut Vec<openai::SummaryText>,
                                 encrypted: &mut Option<String>,
                                 has_reasoning: &mut bool,
                                 id_used: &mut bool,
                                 id: &Option<String>| {
                                    if *has_reasoning {
                                        let use_id_inner =
                                        |used: &mut bool, id: &Option<String>| -> Option<String> {
                                            if *used {
                                                None
                                            } else {
                                                *used = true;
                                                id.clone()
                                            }
                                        };
                                        result.push(openai::OutputItem {
                                            output_item_type: Some(
                                                openai::OutputItemType::Reasoning,
                                            ),
                                            content: Some(vec![]),
                                            summary: Some(std::mem::take(summaries)),
                                            encrypted_content: encrypted.take(),
                                            id: use_id_inner(id_used, id),
                                            ..Default::default()
                                        });
                                        *has_reasoning = false;
                                    }
                                };

                            for part in parts {
                                match part {
                                    AssistantContentPart::Text(text_part) => {
                                        // Flush any pending reasoning before text
                                        flush_reasoning(
                                            &mut result,
                                            &mut pending_reasoning_summaries,
                                            &mut pending_encrypted_content,
                                            &mut has_pending_reasoning,
                                            &mut id_used,
                                            &id,
                                        );
                                        // Extract annotations, logprobs, and phase from
                                        // provider_options using a typed struct so we avoid raw
                                        // Value field access. Default annotations to Some(vec![])
                                        // so the Responses API output always has the required array.
                                        #[derive(serde::Deserialize, Default)]
                                        struct TextPartProviderOpts {
                                            annotations: Option<Vec<openai::Annotation>>,
                                            logprobs: Option<Vec<openai::LogProbability>>,
                                            phase: Option<openai::MessagePhase>,
                                        }
                                        let (annotations, logprobs, phase) = if let Some(ref opts) =
                                            text_part.provider_options
                                        {
                                            let parsed =
                                                serde_json::from_value::<TextPartProviderOpts>(
                                                    serde_json::Value::Object(opts.options.clone()),
                                                )
                                                .unwrap_or_default();
                                            (
                                                parsed.annotations.or(Some(vec![])),
                                                parsed.logprobs,
                                                parsed.phase,
                                            )
                                        } else {
                                            (Some(vec![]), None, None)
                                        };
                                        result.push(openai::OutputItem {
                                            output_item_type: Some(openai::OutputItemType::Message),
                                            role: Some(openai::RoleEnum::Assistant),
                                            content: Some(vec![openai::OutputMessageContent {
                                                output_message_content_type:
                                                    openai::ContentType::OutputText,
                                                text: Some(text_part.text),
                                                annotations,
                                                logprobs,
                                                refusal: None,
                                            }]),
                                            id: use_id(&mut id_used, &id),
                                            status: Some(openai::FunctionCallItemStatus::Completed),
                                            phase,
                                            ..Default::default()
                                        });
                                    }
                                    AssistantContentPart::Reasoning {
                                        text,
                                        encrypted_content,
                                    } => {
                                        // Accumulate reasoning summaries
                                        has_pending_reasoning = true;
                                        if !text.is_empty() {
                                            pending_reasoning_summaries.push(openai::SummaryText {
                                                text,
                                                summary_text_type: openai::SummaryType::SummaryText,
                                            });
                                        }
                                        if encrypted_content.is_some() {
                                            pending_encrypted_content = encrypted_content;
                                        }
                                    }
                                    AssistantContentPart::ToolCall {
                                        tool_call_id,
                                        tool_name,
                                        arguments,
                                        status,
                                        provider_options,
                                        provider_executed,
                                        caller,
                                        encrypted_content: _,
                                    } => {
                                        // Flush any pending reasoning before tool call
                                        flush_reasoning(
                                            &mut result,
                                            &mut pending_reasoning_summaries,
                                            &mut pending_encrypted_content,
                                            &mut has_pending_reasoning,
                                            &mut id_used,
                                            &id,
                                        );
                                        let provider_options_view =
                                            openai_tool_call_provider_options_view(
                                                &provider_options,
                                            );
                                        let namespace = provider_options_view
                                            .as_ref()
                                            .and_then(|opts| opts.namespace.clone());
                                        let output_item_status = status
                                            .as_deref()
                                            .map(|status| {
                                                function_call_item_status_from_string(
                                                    status,
                                                    "Responses function call status",
                                                )
                                            })
                                            .transpose()?
                                            .unwrap_or(openai::FunctionCallItemStatus::Completed);

                                        if provider_executed == Some(true) {
                                            // Built-in tool: convert to appropriate OutputItem type
                                            let args_value = match &arguments {
                                                ToolCallArguments::Valid(map) => {
                                                    serde_json::Value::Object(map.clone())
                                                }
                                                ToolCallArguments::Invalid(s)
                                                | ToolCallArguments::Custom(s) => {
                                                    serde_json::Value::String(s.clone())
                                                }
                                            };

                                            let item = match &tool_name[..] {
                                                "code_interpreter" => openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::CodeInterpreterCall,
                                                    ),
                                                    id: Some(tool_call_id),
                                                    code: parse_builtin_field(
                                                        &args_value,
                                                        "code",
                                                        "code_interpreter",
                                                    )?,
                                                    container_id: parse_builtin_field(
                                                        &args_value,
                                                        "container_id",
                                                        "code_interpreter",
                                                    )?,
                                                    outputs: parse_builtin_field(
                                                        &args_value,
                                                        "outputs",
                                                        "code_interpreter",
                                                    )?,
                                                    status: parse_builtin_field(
                                                        &args_value,
                                                        "status",
                                                        "code_interpreter",
                                                    )?,
                                                    ..Default::default()
                                                },
                                                "web_search" => openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::WebSearchCall,
                                                    ),
                                                    id: Some(tool_call_id),
                                                    action: parse_builtin_field(
                                                        &args_value,
                                                        "action",
                                                        "web_search",
                                                    )?,
                                                    queries: parse_builtin_field(
                                                        &args_value,
                                                        "queries",
                                                        "web_search",
                                                    )?,
                                                    status: parse_builtin_field(
                                                        &args_value,
                                                        "status",
                                                        "web_search",
                                                    )?,
                                                    ..Default::default()
                                                },
                                                "file_search" => openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::FileSearchCall,
                                                    ),
                                                    id: Some(tool_call_id),
                                                    queries: parse_builtin_field(
                                                        &args_value,
                                                        "queries",
                                                        "file_search",
                                                    )?,
                                                    results: parse_builtin_field(
                                                        &args_value,
                                                        "results",
                                                        "file_search",
                                                    )?,
                                                    status: parse_builtin_field(
                                                        &args_value,
                                                        "status",
                                                        "file_search",
                                                    )?,
                                                    ..Default::default()
                                                },
                                                "computer" => openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::ComputerCall,
                                                    ),
                                                    id: Some(tool_call_id),
                                                    action: parse_builtin_field(
                                                        &args_value,
                                                        "action",
                                                        "computer",
                                                    )?,
                                                    status: parse_builtin_field(
                                                        &args_value,
                                                        "status",
                                                        "computer",
                                                    )?,
                                                    ..Default::default()
                                                },
                                                "image_generation" => openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::ImageGenerationCall,
                                                    ),
                                                    id: Some(tool_call_id),
                                                    result: parse_builtin_field(
                                                        &args_value,
                                                        "result",
                                                        "image_generation",
                                                    )?,
                                                    status: parse_builtin_field(
                                                        &args_value,
                                                        "status",
                                                        "image_generation",
                                                    )?,
                                                    ..Default::default()
                                                },
                                                "local_shell" => openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::LocalShellCall,
                                                    ),
                                                    id: Some(tool_call_id),
                                                    action: parse_builtin_field(
                                                        &args_value,
                                                        "action",
                                                        "local_shell",
                                                    )?,
                                                    status: parse_builtin_field(
                                                        &args_value,
                                                        "status",
                                                        "local_shell",
                                                    )?,
                                                    ..Default::default()
                                                },
                                                "mcp_call" => openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::McpCall,
                                                    ),
                                                    id: Some(tool_call_id),
                                                    server_label: parse_builtin_field(
                                                        &args_value,
                                                        "server_label",
                                                        "mcp_call",
                                                    )?,
                                                    status: parse_builtin_field(
                                                        &args_value,
                                                        "status",
                                                        "mcp_call",
                                                    )?,
                                                    ..Default::default()
                                                },
                                                "mcp_list_tools" => openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::McpListTools,
                                                    ),
                                                    id: Some(tool_call_id),
                                                    server_label: parse_builtin_field(
                                                        &args_value,
                                                        "server_label",
                                                        "mcp_list_tools",
                                                    )?,
                                                    tools: parse_builtin_field(
                                                        &args_value,
                                                        "tools",
                                                        "mcp_list_tools",
                                                    )?,
                                                    status: parse_builtin_field(
                                                        &args_value,
                                                        "status",
                                                        "mcp_list_tools",
                                                    )?,
                                                    ..Default::default()
                                                },
                                                "mcp_approval_request" => openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::McpApprovalRequest,
                                                    ),
                                                    id: Some(tool_call_id),
                                                    status: parse_builtin_field(
                                                        &args_value,
                                                        "status",
                                                        "mcp_approval_request",
                                                    )?,
                                                    ..Default::default()
                                                },
                                                _ => {
                                                    // Unknown provider-executed tool - fall back to FunctionCall
                                                    openai::OutputItem {
                                                        output_item_type: Some(
                                                            openai::OutputItemType::FunctionCall,
                                                        ),
                                                        call_id: Some(tool_call_id),
                                                        name: Some(tool_name),
                                                        namespace,
                                                        caller: caller.map(Into::into),
                                                        arguments: Some(serde_json::Value::String(
                                                            arguments.to_string(),
                                                        )),
                                                        status: Some(output_item_status),
                                                        ..Default::default()
                                                    }
                                                }
                                            };
                                            result.push(item);
                                        } else if let ToolCallArguments::Custom(input) = arguments {
                                            result.push(openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::CustomToolCall,
                                                ),
                                                id: use_id(&mut id_used, &id),
                                                call_id: Some(tool_call_id),
                                                name: Some(tool_name),
                                                namespace,
                                                caller: caller.map(Into::into),
                                                input: Some(input),
                                                status: Some(output_item_status),
                                                ..Default::default()
                                            });
                                        } else {
                                            // Regular function call
                                            result.push(openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::FunctionCall,
                                                ),
                                                id: use_id(&mut id_used, &id),
                                                call_id: Some(tool_call_id),
                                                name: Some(tool_name),
                                                namespace,
                                                caller: caller.map(Into::into),
                                                arguments: Some(serde_json::Value::String(
                                                    arguments.to_string(),
                                                )),
                                                status: Some(output_item_status),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                    AssistantContentPart::Program {
                                        call_id,
                                        code,
                                        fingerprint,
                                        id: program_id,
                                    } => {
                                        flush_reasoning(
                                            &mut result,
                                            &mut pending_reasoning_summaries,
                                            &mut pending_encrypted_content,
                                            &mut has_pending_reasoning,
                                            &mut id_used,
                                            &id,
                                        );
                                        result.push(openai::OutputItem {
                                            output_item_type: Some(openai::OutputItemType::Program),
                                            id: program_id.clone(),
                                            call_id: Some(call_id),
                                            code: Some(code),
                                            fingerprint,
                                            ..Default::default()
                                        });
                                    }
                                    AssistantContentPart::ProgramOutput {
                                        call_id,
                                        result: program_result,
                                        status,
                                        id: program_output_id,
                                    } => {
                                        flush_reasoning(
                                            &mut result,
                                            &mut pending_reasoning_summaries,
                                            &mut pending_encrypted_content,
                                            &mut has_pending_reasoning,
                                            &mut id_used,
                                            &id,
                                        );
                                        result.push(openai::OutputItem {
                                            output_item_type: Some(
                                                openai::OutputItemType::ProgramOutput,
                                            ),
                                            id: program_output_id.clone(),
                                            call_id: Some(call_id),
                                            result: Some(program_result),
                                            status: Some(
                                                serde_json::from_value(serde_json::Value::String(
                                                    status,
                                                ))
                                                .map_err(|e| ConvertError::InvalidEnumValue {
                                                    type_name: "FunctionCallItemStatus",
                                                    value: e.to_string(),
                                                })?,
                                            ),
                                            ..Default::default()
                                        });
                                    }
                                    AssistantContentPart::ToolDiscoveryCall {
                                        tool_call_id,
                                        discovery_tool_name: _,
                                        query,
                                        arguments,
                                        status,
                                        execution,
                                        ..
                                    } => {
                                        flush_reasoning(
                                            &mut result,
                                            &mut pending_reasoning_summaries,
                                            &mut pending_encrypted_content,
                                            &mut has_pending_reasoning,
                                            &mut id_used,
                                            &id,
                                        );
                                        result.push(tool_discovery::output_call_from_universal(
                                            tool_call_id,
                                            query,
                                            arguments,
                                            status,
                                            execution,
                                            use_id(&mut id_used, &id),
                                        ));
                                    }
                                    // Skip File and ToolResult variants as they don't map to OutputItems
                                    _ => {}
                                }
                            }
                            // Flush any remaining pending reasoning at the end
                            flush_reasoning(
                                &mut result,
                                &mut pending_reasoning_summaries,
                                &mut pending_encrypted_content,
                                &mut has_pending_reasoning,
                                &mut id_used,
                                &id,
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        // Fill in placeholder IDs for any items that don't have one.
        // Responses API requires every output item to have an id; sources like
        // Anthropic carry only a response-level id, not per-item ids.
        for (i, item) in result.iter_mut().enumerate() {
            if item.id.is_none() {
                item.id = Some(format!("msg_{}_item_{}", PLACEHOLDER_ID, i));
            }
        }

        Ok(result)
    }
}

/// Helper function to convert InputContent to OutputMessageContent
fn convert_input_content_to_output_message_content(
    input_content: openai::InputContent,
) -> Result<openai::OutputMessageContent, ConvertError> {
    match input_content.input_content_type {
        openai::InputItemContentListType::OutputText
        | openai::InputItemContentListType::InputText => Ok(openai::OutputMessageContent {
            output_message_content_type: openai::ContentType::OutputText,
            text: input_content.text,
            annotations: input_content.annotations,
            logprobs: input_content.logprobs,
            refusal: input_content.refusal,
        }),
        openai::InputItemContentListType::ReasoningText => Ok(openai::OutputMessageContent {
            output_message_content_type: openai::ContentType::OutputText,
            text: input_content.text,
            annotations: input_content.annotations,
            logprobs: input_content.logprobs,
            refusal: input_content.refusal,
        }),
        _ => {
            // For other content types, try to preserve as much information as possible
            Ok(openai::OutputMessageContent {
                output_message_content_type: openai::ContentType::OutputText,
                text: input_content.text,
                annotations: input_content.annotations,
                logprobs: input_content.logprobs,
                refusal: input_content.refusal,
            })
        }
    }
}

/// Default implementation for OutputItem
impl Default for openai::OutputItem {
    fn default() -> Self {
        Self {
            content: None,
            id: None,
            phase: None,
            role: None,
            status: None,
            output_item_type: None, // Don't add type field if original didn't have it
            queries: None,
            results: None,
            arguments: None,
            call_id: None,
            name: None,
            namespace: None,
            action: None,
            actions: None,
            pending_safety_checks: None,
            acknowledged_safety_checks: None,
            encrypted_content: None,
            summary: None,
            result: None,
            code: None,
            fingerprint: None,
            caller: None,
            container_id: None,
            outputs: None,
            created_by: None,
            execution: None,
            environment: None,
            max_output_length: None,
            operation: None,
            error: None,
            output: None,
            server_label: None,
            tools: None,
            approval_request_id: None,
            approve: None,
            reason: None,
            input: None,
            request_id: None,
        }
    }
}

/// Helper function to convert OutputMessageContent to InputContent
fn convert_output_message_content_to_input_content(
    output_content: openai::OutputMessageContent,
) -> Result<openai::InputContent, ConvertError> {
    match output_content.output_message_content_type {
        openai::ContentType::OutputText => Ok(openai::InputContent {
            input_content_type: openai::InputItemContentListType::OutputText,
            text: output_content.text,
            annotations: output_content.annotations,
            logprobs: output_content.logprobs,
            refusal: output_content.refusal,
            ..Default::default()
        }),
        // TODO: Handle other content types like tool calls when they're properly supported
        _ => {
            // For other content types, try to preserve as much information as possible
            Ok(openai::InputContent {
                input_content_type: openai::InputItemContentListType::OutputText, // Default fallback
                text: output_content.text,
                annotations: output_content.annotations,
                logprobs: output_content.logprobs,
                refusal: output_content.refusal,
                ..Default::default()
            })
        }
    }
}

// ============================================================================
// Chat Completion Conversions
// ============================================================================

/// Convert ChatCompletionRequestMessageExt to universal Message
impl TryFromLLM<ChatCompletionRequestMessageExt> for Message {
    type Error = ConvertError;

    fn try_from(msg: ChatCompletionRequestMessageExt) -> Result<Self, Self::Error> {
        match msg.role {
            openai::ChatCompletionRequestMessageRole::System => {
                let content =
                    chat_completion_content_to_user_content(msg.content, msg.cache_control)?;
                Ok(Message::System { content })
            }
            openai::ChatCompletionRequestMessageRole::User => {
                let content =
                    chat_completion_content_to_user_content(msg.content, msg.cache_control)?;
                Ok(Message::User { content })
            }
            openai::ChatCompletionRequestMessageRole::Assistant => {
                let mut content_parts: Vec<AssistantContentPart> = Vec::new();

                // Add reasoning FIRST if present (natural model output order)
                // Note: We preserve empty reasoning strings because the presence of the
                // reasoning field indicates reasoning occurred (content may be hidden/summarized)
                if let Some(reasoning) = msg.reasoning.and_then(|r| r.into_text()) {
                    content_parts.push(AssistantContentPart::Reasoning {
                        text: reasoning,
                        encrypted_content: msg.reasoning_signature.clone(),
                    });
                }

                // Add text content if present
                match msg.content {
                    Some(ChatCompletionRequestMessageContentExt::String(text))
                        if !text.is_empty() =>
                    {
                        content_parts.push(AssistantContentPart::Text(TextContentPart {
                            text,
                            encrypted_content: None,
                            cache_control: cache_control_from_value(msg.cache_control),
                            provider_options: None,
                        }));
                    }
                    Some(ChatCompletionRequestMessageContentExt::String(_)) => {}
                    Some(ChatCompletionRequestMessageContentExt::Parts(parts)) => {
                        let assistant_parts: Result<Vec<_>, _> = parts
                            .into_iter()
                            .map(|part| {
                                match part.base.chat_completion_request_message_content_part_type {
                                    openai::PurpleType::Text => {
                                        if let Some(text) = part.base.text {
                                            Ok(AssistantContentPart::Text(TextContentPart {
                                                text,
                                                encrypted_content: None,
                                                cache_control: cache_control_from_value(
                                                    part.cache_control,
                                                ),
                                                provider_options: None,
                                            }))
                                        } else {
                                            Err(ConvertError::MissingRequiredField {
                                                field: "text".to_string(),
                                            })
                                        }
                                    }
                                    _ => Err(ConvertError::UnsupportedInputType {
                                        type_info: format!(
                                            "ChatCompletionRequestMessageContentPart type: {:?}",
                                            part.base
                                                .chat_completion_request_message_content_part_type
                                        ),
                                    }),
                                }
                            })
                            .collect();
                        content_parts.extend(assistant_parts?);
                    }
                    None => {}
                }

                // Add tool calls if present
                if let Some(tool_calls) = msg.tool_calls {
                    for tool_call in tool_calls {
                        if let Some(function) = tool_call.function {
                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_call.id,
                                tool_name: function.name,
                                arguments: function.arguments.into(),
                                encrypted_content: msg.reasoning_signature.clone(),
                                provider_options: None,
                                status: None,
                                caller: None,
                                provider_executed: None,
                            });
                        }
                    }
                }

                let content = assistant_content_from_parts(content_parts);

                Ok(Message::Assistant { content, id: None })
            }
            openai::ChatCompletionRequestMessageRole::Developer => {
                let content =
                    chat_completion_content_to_user_content(msg.content, msg.cache_control)?;
                Ok(Message::Developer { content })
            }
            openai::ChatCompletionRequestMessageRole::Tool => {
                // Tool messages should extract tool_call_id and content
                let content_text = match msg.content {
                    Some(ChatCompletionRequestMessageContentExt::String(text)) => text,
                    Some(ChatCompletionRequestMessageContentExt::Parts(mut arr)) => {
                        if arr.len() != 1 {
                            return Err(ConvertError::UnsupportedInputType {
                                type_info: format!(
                                    "Tool messages must have a single array element (found {})",
                                    arr.len()
                                ),
                            });
                        }
                        let part = arr.remove(0);
                        if let Some(text) = part.base.text {
                            text
                        } else {
                            return Err(ConvertError::UnsupportedInputType {
                                type_info: "Tool content part must have text".to_string(),
                            });
                        }
                    }
                    None => {
                        return Err(ConvertError::MissingRequiredField {
                            field: "content".to_string(),
                        })
                    }
                };

                let tool_call_id =
                    msg.tool_call_id
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "tool_call_id".to_string(),
                        })?;

                // Convert to universal Tool message format
                // Try to parse as JSON, fall back to string if parsing fails
                let output_value = serde_json::from_str(&content_text)
                    .unwrap_or(serde_json::Value::String(content_text));

                let tool_result = ToolResultContentPart {
                    tool_call_id: tool_call_id.clone(),
                    tool_name: String::new(), // OpenAI doesn't provide tool name in tool messages
                    output: output_value,
                    custom_tool_call: None,
                    caller: None,
                    provider_options: None,
                };

                Ok(Message::Tool {
                    content: vec![ToolContentPart::ToolResult(tool_result)],
                })
            }
            openai::ChatCompletionRequestMessageRole::Function => {
                // Legacy function-role messages: preserve as tool output-like content.
                let content_text = match msg.content {
                    Some(ChatCompletionRequestMessageContentExt::String(text)) => text,
                    Some(ChatCompletionRequestMessageContentExt::Parts(mut arr)) => {
                        if arr.len() != 1 {
                            String::new()
                        } else {
                            arr.remove(0).base.text.unwrap_or_default()
                        }
                    }
                    None => String::new(),
                };
                let name = msg.name.unwrap_or_default();
                Ok(Message::Tool {
                    content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                        tool_call_id: name.clone(),
                        tool_name: name,
                        output: serde_json::Value::String(content_text),
                        custom_tool_call: None,
                        caller: None,
                        provider_options: None,
                    })],
                })
            }
        }
    }
}

/// Convert ChatCompletionRequestMessageContentPart to UserContentPart
impl TryFromLLM<openai::ChatCompletionRequestMessageContentPart> for UserContentPart {
    type Error = ConvertError;

    fn try_from(
        part: openai::ChatCompletionRequestMessageContentPart,
    ) -> Result<Self, Self::Error> {
        match part.chat_completion_request_message_content_part_type {
            openai::PurpleType::Text => {
                if let Some(text) = part.text {
                    Ok(UserContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }))
                } else {
                    Err(ConvertError::MissingRequiredField {
                        field: "text".to_string(),
                    })
                }
            }
            openai::PurpleType::ImageUrl => {
                if let Some(image_url) = part.image_url {
                    // Parse data URLs to extract raw base64, keep HTTP URLs as-is
                    let (image_data, media_type) =
                        if let Some(block) = parse_base64_data_url(&image_url.url) {
                            // Data URL: extract raw base64 and media type
                            (block.data, Some(block.media_type))
                        } else {
                            // HTTP URL or other: keep as-is, no media type
                            (image_url.url.clone(), None)
                        };

                    Ok(UserContentPart::Image {
                        image: serde_json::Value::String(image_data),
                        media_type,
                        provider_options: None,
                    })
                } else {
                    Err(ConvertError::MissingRequiredField {
                        field: "image_url".to_string(),
                    })
                }
            }
            openai::PurpleType::File => {
                let file = part
                    .file
                    .ok_or_else(|| ConvertError::MissingRequiredField {
                        field: "file".to_string(),
                    })?;
                let (payload, filename) = universal_file_payload_from_openai(
                    file.file_data,
                    None,
                    file.file_id,
                    file.filename,
                )?;

                Ok(UserContentPart::File {
                    data: payload.data,
                    filename,
                    media_type: payload.media_type,
                    provider_options: None,
                })
            }
            _ => Err(ConvertError::UnsupportedInputType {
                type_info: format!(
                    "ChatCompletionRequestMessageContentPart type: {:?}",
                    part.chat_completion_request_message_content_part_type
                ),
            }),
        }
    }
}

impl TryFromLLM<ChatCompletionRequestMessageContentPartExt> for UserContentPart {
    type Error = ConvertError;

    fn try_from(part: ChatCompletionRequestMessageContentPartExt) -> Result<Self, Self::Error> {
        match part.base.chat_completion_request_message_content_part_type {
            openai::PurpleType::Text => {
                if let Some(text) = part.base.text {
                    Ok(UserContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        cache_control: cache_control_from_value(part.cache_control),
                        provider_options: None,
                    }))
                } else {
                    Err(ConvertError::MissingRequiredField {
                        field: "text".to_string(),
                    })
                }
            }
            openai::PurpleType::ImageUrl => {
                if let Some(image_url) = part.base.image_url {
                    let (image_data, media_type) =
                        if let Some(block) = parse_base64_data_url(&image_url.url) {
                            (block.data, Some(block.media_type))
                        } else {
                            (image_url.url.clone(), None)
                        };

                    Ok(UserContentPart::Image {
                        image: serde_json::Value::String(image_data),
                        media_type,
                        provider_options: None,
                    })
                } else {
                    Err(ConvertError::MissingRequiredField {
                        field: "image_url".to_string(),
                    })
                }
            }
            openai::PurpleType::File => {
                let file = part
                    .base
                    .file
                    .ok_or_else(|| ConvertError::MissingRequiredField {
                        field: "file".to_string(),
                    })?;
                let (payload, filename) = universal_file_payload_from_openai(
                    file.file_data,
                    None,
                    file.file_id,
                    file.filename,
                )?;

                Ok(UserContentPart::File {
                    data: payload.data,
                    filename,
                    media_type: payload.media_type,
                    provider_options: None,
                })
            }
            _ => Err(ConvertError::UnsupportedInputType {
                type_info: format!(
                    "ChatCompletionRequestMessageContentPart type: {:?}",
                    part.base.chat_completion_request_message_content_part_type
                ),
            }),
        }
    }
}

/// Convert universal Message to ChatCompletionRequestMessage
impl TryFromLLM<Message> for ChatCompletionRequestMessageExt {
    type Error = ConvertError;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg {
            Message::System { content } => Ok(ChatCompletionRequestMessageExt {
                role: openai::ChatCompletionRequestMessageRole::System,
                content: Some(convert_user_content_to_chat_completion_content(content)?),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                audio: None,
                function_call: None,
                refusal: None,
                cache_control: None,
                reasoning: None,
                reasoning_signature: None,
            }),
            Message::Developer { content } => Ok(ChatCompletionRequestMessageExt {
                role: openai::ChatCompletionRequestMessageRole::Developer,
                content: Some(convert_user_content_to_chat_completion_content(content)?),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                audio: None,
                function_call: None,
                refusal: None,
                cache_control: None,
                reasoning: None,
                reasoning_signature: None,
            }),
            Message::User { content } => Ok(ChatCompletionRequestMessageExt {
                role: openai::ChatCompletionRequestMessageRole::User,
                content: Some(convert_user_content_to_chat_completion_content(content)?),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                audio: None,
                function_call: None,
                refusal: None,
                cache_control: None,
                reasoning: None,
                reasoning_signature: None,
            }),
            Message::Assistant { content, id: _ } => {
                let (text_content, tool_calls, reasoning, reasoning_signature) =
                    extract_content_tool_calls_and_reasoning(content)?;

                Ok(ChatCompletionRequestMessageExt {
                    role: openai::ChatCompletionRequestMessageRole::Assistant,
                    content: text_content,
                    name: None,
                    tool_calls,
                    tool_call_id: None,
                    audio: None,
                    function_call: None,
                    refusal: None,
                    cache_control: None,
                    reasoning: reasoning.map(ChatCompletionRequestReasoning::String),
                    reasoning_signature,
                })
            }
            Message::Tool { content } => {
                let part = content.into_iter().next().ok_or_else(|| {
                    ConvertError::MissingRequiredField {
                        field: "tool_result".to_string(),
                    }
                })?;
                match part {
                    ToolContentPart::ToolResult(result) => {
                        tool_result_to_chat_completion_message(result)
                    }
                    ToolContentPart::ToolDiscoveryResult(result) => {
                        tool_discovery_result_to_chat_completion_message(result)
                    }
                }
            }
            Message::AdditionalTools { .. } => Err(ConvertError::UnsupportedMapping {
                from: "Message::AdditionalTools".to_string(),
                to: "ChatCompletionRequestMessage",
            }),
        }
    }
}

/// Convert `Vec<Message>` to `Vec<ChatCompletionRequestMessageExt>`, expanding
/// any `Message::Tool` with multiple results into one message per result.
///
/// Anthropic (and others) group parallel tool results into a single
/// `Message::Tool { content: [result1, result2] }`, but OpenAI Chat Completions
/// requires a separate `role: "tool"` message for each result.
pub(crate) fn messages_to_chat_completion_messages(
    messages: Vec<Message>,
) -> Result<Vec<ChatCompletionRequestMessageExt>, ConvertError> {
    let mut result = Vec::new();
    for msg in messages {
        match msg {
            Message::Tool { content } => {
                for part in content {
                    match part {
                        ToolContentPart::ToolResult(tool_result) => {
                            result.push(tool_result_to_chat_completion_message(tool_result)?);
                        }
                        ToolContentPart::ToolDiscoveryResult(discovery_result) => {
                            result.push(tool_discovery_result_to_chat_completion_message(
                                discovery_result,
                            )?);
                        }
                    }
                }
            }
            other => result
                .push(<ChatCompletionRequestMessageExt as TryFromLLM<Message>>::try_from(other)?),
        }
    }
    Ok(result)
}

/// Convert a single tool result into a chat completions tool-role message.
pub(crate) fn tool_result_to_chat_completion_message(
    result: ToolResultContentPart,
) -> Result<ChatCompletionRequestMessageExt, ConvertError> {
    let content_string = match &result.output {
        serde_json::Value::String(s) => s.clone(),
        other => {
            serde_json::to_string(other).map_err(|e| ConvertError::JsonSerializationFailed {
                field: "tool_result_content".to_string(),
                error: e.to_string(),
            })?
        }
    };
    Ok(ChatCompletionRequestMessageExt {
        role: openai::ChatCompletionRequestMessageRole::Tool,
        content: Some(ChatCompletionRequestMessageContentExt::String(
            content_string,
        )),
        name: None,
        tool_calls: None,
        tool_call_id: Some(result.tool_call_id),
        audio: None,
        function_call: None,
        refusal: None,
        cache_control: None,
        reasoning: None,
        reasoning_signature: None,
    })
}

fn tool_discovery_result_to_chat_completion_message(
    result: ToolDiscoveryResultContentPart,
) -> Result<ChatCompletionRequestMessageExt, ConvertError> {
    let mut content = serde_json::Map::new();
    content.insert(
        "discovery_tool_name".to_string(),
        serde_json::Value::String(result.discovery_tool_name),
    );
    content.insert(
        "tools".to_string(),
        serde_json::to_value(result.tools).map_err(|e| ConvertError::JsonSerializationFailed {
            field: "tool_discovery_result.tools".to_string(),
            error: e.to_string(),
        })?,
    );
    if let Some(status) = result.status {
        content.insert("status".to_string(), serde_json::Value::String(status));
    }
    if let Some(execution) = result.execution {
        content.insert(
            "execution".to_string(),
            serde_json::Value::String(execution),
        );
    }

    let content_string =
        serde_json::to_string(&serde_json::Value::Object(content)).map_err(|e| {
            ConvertError::JsonSerializationFailed {
                field: "tool_discovery_result_content".to_string(),
                error: e.to_string(),
            }
        })?;

    Ok(ChatCompletionRequestMessageExt {
        role: openai::ChatCompletionRequestMessageRole::Tool,
        content: Some(ChatCompletionRequestMessageContentExt::String(
            content_string,
        )),
        name: None,
        tool_calls: None,
        tool_call_id: Some(result.tool_call_id),
        audio: None,
        function_call: None,
        refusal: None,
        cache_control: None,
        reasoning: None,
        reasoning_signature: None,
    })
}

/// Convert UserContent to ChatCompletionRequestMessageContent
fn convert_user_content_to_chat_completion_content(
    content: UserContent,
) -> Result<ChatCompletionRequestMessageContentExt, ConvertError> {
    match content {
        UserContent::String(text) => Ok(ChatCompletionRequestMessageContentExt::String(text)),
        UserContent::Array(parts) => {
            let chat_parts: Result<Vec<_>, _> = parts
                .into_iter()
                .map(convert_user_content_part_to_chat_completion_part_ext)
                .collect();
            Ok(ChatCompletionRequestMessageContentExt::Parts(chat_parts?))
        }
    }
}

fn convert_user_content_part_to_chat_completion_part_ext(
    part: UserContentPart,
) -> Result<ChatCompletionRequestMessageContentPartExt, ConvertError> {
    let cache_control = match &part {
        UserContentPart::Text(text_part) => cache_control_to_value(text_part.cache_control.clone()),
        _ => None,
    };
    Ok(ChatCompletionRequestMessageContentPartExt {
        base: convert_user_content_part_to_chat_completion_part(part)?,
        cache_control,
    })
}

/// Convert UserContentPart to ChatCompletionRequestMessageContentPart
fn convert_user_content_part_to_chat_completion_part(
    part: UserContentPart,
) -> Result<openai::ChatCompletionRequestMessageContentPart, ConvertError> {
    match part {
        UserContentPart::Text(text_part) => Ok(openai::ChatCompletionRequestMessageContentPart {
            prompt_cache_breakpoint: None,
            text: Some(text_part.text),
            chat_completion_request_message_content_part_type: openai::PurpleType::Text,
            image_url: None,
            input_audio: None,
            file: None,
            refusal: None,
        }),
        UserContentPart::Image {
            image,
            media_type,
            provider_options: _,
        } => {
            // Convert image to ImageUrl format
            let image_str = match image {
                serde_json::Value::String(url) => url,
                _ => {
                    return Err(ConvertError::UnsupportedInputType {
                        type_info: format!(
                            "Image must be string URL for ChatCompletion, got: {:?}",
                            image
                        ),
                    })
                }
            };

            // If we have raw base64 data (not a URL) and media_type, create a proper data URL
            let url = if !image_str.starts_with("data:")
                && !image_str.starts_with("http://")
                && !image_str.starts_with("https://")
            {
                // Assume raw base64 data - create data URL with media_type
                let mt = media_type.as_deref().unwrap_or("image/jpeg");
                format!("data:{};base64,{}", mt, image_str)
            } else {
                image_str
            };

            Ok(openai::ChatCompletionRequestMessageContentPart {
                prompt_cache_breakpoint: None,
                text: None,
                chat_completion_request_message_content_part_type: openai::PurpleType::ImageUrl,
                image_url: Some(openai::ImageUrl { url, detail: None }),
                input_audio: None,
                file: None,
                refusal: None,
            })
        }
        UserContentPart::File {
            media_type,
            ..
        } => {
            Err(ConvertError::UnsupportedInputType {
                type_info: format!(
                    "UserContentPart::File (media_type: {}) is not supported by OpenAI ChatCompletions. \
OpenAI ChatCompletions file inputs require provider-specific file payloads (for example PDF data URLs), \
and Anthropic document blocks do not map safely.",
                    media_type
                ),
            })
        }
    }
}

type ExtractedContentResult = (
    Option<ChatCompletionRequestMessageContentExt>,
    Option<Vec<openai::ToolCall>>,
    Option<String>,
    Option<String>, // reasoning_signature
);

/// Extract text content, tool calls, reasoning, and reasoning_signature from AssistantContent
fn extract_content_tool_calls_and_reasoning(
    content: AssistantContent,
) -> Result<ExtractedContentResult, ConvertError> {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();
    let mut reasoning_parts = Vec::new();
    let mut reasoning_signature: Option<String> = None;

    match content {
        AssistantContent::String(text) => {
            return Ok((
                Some(ChatCompletionRequestMessageContentExt::String(text)),
                None,
                None,
                None,
            ));
        }
        AssistantContent::Array(parts) => {
            for part in parts {
                match part {
                    AssistantContentPart::Text(text_part) => {
                        text_parts.push(text_part);
                    }
                    AssistantContentPart::Reasoning {
                        text,
                        encrypted_content,
                    } => {
                        reasoning_parts.push(text);
                        // Take the first signature if multiple reasoning blocks exist
                        if reasoning_signature.is_none() {
                            reasoning_signature = encrypted_content;
                        }
                    }
                    AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments,
                        encrypted_content,
                        ..
                    } => {
                        if reasoning_signature.is_none() {
                            reasoning_signature = encrypted_content.clone();
                        }
                        tool_calls.push(openai::ToolCall {
                            id: tool_call_id,
                            tool_call_type: openai::FluffyType::Function,
                            function: Some(openai::PurpleFunction {
                                name: tool_name,
                                arguments: arguments.to_string(),
                            }),
                            custom: None,
                        });
                    }
                    AssistantContentPart::ToolDiscoveryCall {
                        tool_call_id,
                        discovery_tool_name,
                        query,
                        arguments,
                        ..
                    } => {
                        tool_calls.push(openai::ToolCall {
                            id: tool_call_id,
                            tool_call_type: openai::FluffyType::Function,
                            function: Some(openai::PurpleFunction {
                                name: discovery_tool_name,
                                arguments: tool_discovery_arguments_to_string(query, arguments)?,
                            }),
                            custom: None,
                        });
                    }
                    _ => {
                        // Handle other content types if needed
                    }
                }
            }
        }
    }

    let text_content = chat_completion_assistant_text_content(text_parts);

    let tool_calls_option = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    let reasoning = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join(""))
    };

    Ok((
        text_content,
        tool_calls_option,
        reasoning,
        reasoning_signature,
    ))
}

fn tool_discovery_arguments_to_string(
    query: Option<String>,
    arguments: Option<serde_json::Value>,
) -> Result<String, ConvertError> {
    let value = arguments.unwrap_or_else(|| match query {
        Some(query) => serde_json::json!({ "query": query }),
        None => serde_json::json!({}),
    });

    match value {
        serde_json::Value::String(text) => Ok(text),
        other => serde_json::to_string(&other).map_err(|e| ConvertError::JsonSerializationFailed {
            field: "tool_discovery_call.arguments".to_string(),
            error: e.to_string(),
        }),
    }
}

fn chat_completion_assistant_text_content(
    text_parts: Vec<TextContentPart>,
) -> Option<ChatCompletionRequestMessageContentExt> {
    if text_parts.is_empty() {
        return None;
    }

    let has_metadata = text_parts.iter().any(|text_part| {
        text_part.cache_control.is_some()
            || text_part.encrypted_content.is_some()
            || text_part.provider_options.is_some()
    });

    if !has_metadata {
        return Some(ChatCompletionRequestMessageContentExt::String(
            text_parts
                .into_iter()
                .map(|text_part| text_part.text)
                .collect::<Vec<_>>()
                .join(""),
        ));
    }

    Some(ChatCompletionRequestMessageContentExt::Parts(
        text_parts
            .into_iter()
            .map(|text_part| ChatCompletionRequestMessageContentPartExt {
                cache_control: cache_control_to_value(text_part.cache_control),
                base: openai::ChatCompletionRequestMessageContentPart {
                    prompt_cache_breakpoint: None,
                    text: Some(text_part.text),
                    chat_completion_request_message_content_part_type: openai::PurpleType::Text,
                    image_url: None,
                    input_audio: None,
                    file: None,
                    refusal: None,
                },
            })
            .collect(),
    ))
}

/// Convert ChatCompletionResponseMessageExt to universal Message
impl TryFromLLM<ChatCompletionResponseMessageExt> for Message {
    type Error = ConvertError;

    fn try_from(msg: ChatCompletionResponseMessageExt) -> Result<Self, Self::Error> {
        match msg.base.role {
            openai::MessageRole::Assistant => {
                let mut content_parts: Vec<AssistantContentPart> = Vec::new();

                // Add reasoning FIRST if present (natural model output order: think first, respond after)
                // Note: We preserve empty reasoning strings because the presence of the
                // reasoning field indicates reasoning occurred (content may be hidden/summarized)
                if let Some(reasoning) = msg.reasoning {
                    content_parts.push(AssistantContentPart::Reasoning {
                        text: reasoning,
                        encrypted_content: msg.reasoning_signature.clone(),
                    });
                }

                // Add text content if present
                if let Some(text) = &msg.base.content {
                    if !text.is_empty() {
                        content_parts.push(AssistantContentPart::Text(TextContentPart {
                            text: text.clone(),
                            encrypted_content: None,
                            cache_control: None,
                            provider_options: None,
                        }));
                    }
                }

                // Add tool calls if present
                if let Some(tool_calls) = &msg.base.tool_calls {
                    for tool_call in tool_calls {
                        if let Some(function) = &tool_call.function {
                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_call.id.clone(),
                                tool_name: function.name.clone(),
                                arguments: function.arguments.clone().into(),
                                encrypted_content: msg.reasoning_signature.clone(),
                                provider_options: None,
                                status: None,
                                caller: None,
                                provider_executed: None,
                            });
                        }
                    }
                }

                let content = assistant_content_from_parts(content_parts);

                Ok(Message::Assistant { content, id: None })
            }
        }
    }
}

/// Convert universal Message to ChatCompletionResponseMessageExt
impl TryFromLLM<&Message> for ChatCompletionResponseMessageExt {
    type Error = ConvertError;

    fn try_from(msg: &Message) -> Result<Self, Self::Error> {
        match msg {
            Message::Assistant { content, id: _ } => {
                let (content_text, tool_calls, reasoning, reasoning_signature) = match content {
                    AssistantContent::String(text) => (Some(text.clone()), None, None, None),
                    AssistantContent::Array(parts) => {
                        // Extract text from parts and concatenate
                        let texts: Vec<String> = parts
                            .iter()
                            .filter_map(|part| match part {
                                AssistantContentPart::Text(text_part) => {
                                    Some(text_part.text.clone())
                                }
                                _ => None,
                            })
                            .collect();

                        // Extract reasoning from parts and concatenate, also capture signature
                        let mut reasonings: Vec<String> = Vec::new();
                        let mut signature: Option<String> = None;
                        for part in parts {
                            if let AssistantContentPart::Reasoning {
                                text,
                                encrypted_content,
                            } = part
                            {
                                reasonings.push(text.clone());
                                merge_reasoning_signature(&mut signature, encrypted_content)?;
                            }
                        }

                        // Extract tool calls from parts
                        let mut tool_calls: Vec<openai::ToolCall> = Vec::new();
                        for part in parts {
                            if let AssistantContentPart::ToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                                encrypted_content,
                                ..
                            } = part
                            {
                                merge_reasoning_signature(&mut signature, encrypted_content)?;
                                tool_calls.push(openai::ToolCall {
                                    id: tool_call_id.clone(),
                                    tool_call_type: openai::FluffyType::Function,
                                    function: Some(openai::PurpleFunction {
                                        name: tool_name.clone(),
                                        arguments: arguments.to_string(),
                                    }),
                                    custom: None,
                                });
                            }
                        }

                        let content_text = if texts.is_empty() {
                            None
                        } else {
                            Some(texts.join(""))
                        };

                        let reasoning = if reasonings.is_empty() {
                            None
                        } else {
                            Some(reasonings.join(""))
                        };

                        let tool_calls_option = if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        };

                        (content_text, tool_calls_option, reasoning, signature)
                    }
                };

                Ok(ChatCompletionResponseMessageExt {
                    base: openai::ChatCompletionResponseMessage {
                        role: openai::MessageRole::Assistant,
                        content: content_text,
                        annotations: Some(vec![]), // Hardcode empty annotations for consistency
                        audio: None,
                        function_call: None,
                        refusal: None,
                        tool_calls,
                    },
                    reasoning,
                    reasoning_signature,
                })
            }
            _ => Err(ConvertError::InvalidEnumValue {
                type_name: "role",
                value: format!("{:?}", msg),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn responses_function_call_invalid_arguments_stays_function_call() {
        let input_items: Vec<openai::InputItem> = serde_json::from_value(json!([
            {
                "type": "function_call",
                "call_id": "call_invalid",
                "name": "get_weather",
                "arguments": "not-json"
            }
        ]))
        .expect("input item should deserialize");

        let messages = <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(input_items)
            .expect("input item should convert to universal");

        let Message::Assistant { content, .. } = &messages[0] else {
            panic!("function call should convert to assistant message");
        };
        let AssistantContent::Array(parts) = content else {
            panic!("assistant content should be array");
        };
        let AssistantContentPart::ToolCall { arguments, .. } = &parts[0] else {
            panic!("content part should be a tool call");
        };
        assert!(matches!(arguments, ToolCallArguments::Invalid(s) if s == "not-json"));

        let roundtrip = universal_to_responses_input(&messages)
            .expect("universal messages should convert back to Responses input");
        assert_eq!(
            roundtrip[0].input_item_type,
            Some(openai::InputItemType::FunctionCall)
        );
        assert_eq!(
            roundtrip[0].arguments,
            Some(openai::Arguments::String("not-json".to_string()))
        );
        assert_eq!(roundtrip[0].input, None);
    }

    #[test]
    fn responses_custom_tool_call_and_output_preserve_item_types() {
        let input_items: Vec<openai::InputItem> = serde_json::from_value(json!([
            {
                "type": "custom_tool_call",
                "call_id": "call_custom",
                "name": "run_raw",
                "input": "raw custom input"
            },
            {
                "type": "custom_tool_call_output",
                "call_id": "call_custom",
                "output": "raw custom output"
            }
        ]))
        .expect("input items should deserialize");

        let messages = <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(input_items)
            .expect("input items should convert to universal");

        let Message::Assistant { content, .. } = &messages[0] else {
            panic!("custom call should convert to assistant message");
        };
        let AssistantContent::Array(parts) = content else {
            panic!("assistant content should be array");
        };
        let AssistantContentPart::ToolCall { arguments, .. } = &parts[0] else {
            panic!("content part should be a tool call");
        };
        assert!(matches!(arguments, ToolCallArguments::Custom(s) if s == "raw custom input"));

        let Message::Tool { content } = &messages[1] else {
            panic!("custom output should convert to tool message");
        };
        let ToolContentPart::ToolResult(result) = &content[0] else {
            panic!("content part should be a tool result");
        };
        assert_eq!(result.custom_tool_call, Some(true));

        let roundtrip = universal_to_responses_input(&messages)
            .expect("universal messages should convert back to Responses input");
        assert_eq!(
            roundtrip[0].input_item_type,
            Some(openai::InputItemType::CustomToolCall)
        );
        assert_eq!(roundtrip[0].input.as_deref(), Some("raw custom input"));
        assert_eq!(roundtrip[0].arguments, None);
        assert_eq!(
            roundtrip[1].input_item_type,
            Some(openai::InputItemType::CustomToolCallOutput)
        );
    }

    #[test]
    fn responses_custom_tool_call_output_roundtrips_as_custom_output_item() {
        let output_items: Vec<openai::OutputItem> = serde_json::from_value(json!([
            {
                "id": "ctc_custom",
                "type": "custom_tool_call",
                "status": "completed",
                "call_id": "call_custom",
                "name": "run_raw",
                "input": "raw custom input"
            }
        ]))
        .expect("output items should deserialize");

        let messages =
            <Vec<Message> as TryFromLLM<Vec<openai::OutputItem>>>::try_from(output_items)
                .expect("output items should convert to universal");
        let roundtrip = <Vec<openai::OutputItem> as TryFromLLM<Vec<Message>>>::try_from(messages)
            .expect("universal messages should convert back to Responses output items");

        assert_eq!(
            roundtrip[0].output_item_type,
            Some(openai::OutputItemType::CustomToolCall)
        );
        assert_eq!(roundtrip[0].input.as_deref(), Some("raw custom input"));
        assert_eq!(roundtrip[0].arguments, None);
    }

    #[test]
    fn responses_function_call_input_preserves_non_completed_status() {
        let input_items: Vec<openai::InputItem> = serde_json::from_value(json!([
            {
                "type": "function_call",
                "call_id": "call_pending",
                "name": "get_weather",
                "arguments": "{}",
                "status": "in_progress"
            }
        ]))
        .expect("input item should deserialize");

        let messages = <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(input_items)
            .expect("input item should convert to universal");

        let Message::Assistant { content, .. } = &messages[0] else {
            panic!("function call should convert to assistant message");
        };
        let AssistantContent::Array(parts) = content else {
            panic!("assistant content should be array");
        };
        let AssistantContentPart::ToolCall {
            provider_options,
            status,
            ..
        } = &parts[0]
        else {
            panic!("content part should be a tool call");
        };
        assert!(provider_options.is_none());
        assert_eq!(status.as_deref(), Some("in_progress"));

        let roundtrip = universal_to_responses_input(&messages)
            .expect("universal messages should convert back to Responses input");
        assert_eq!(
            roundtrip[0].input_item_type,
            Some(openai::InputItemType::FunctionCall)
        );
        assert_eq!(roundtrip[0].status, Some(openai::Status::InProgress));
    }

    #[test]
    fn responses_custom_tool_call_input_preserves_non_completed_status() {
        let input_items: Vec<openai::InputItem> = serde_json::from_value(json!([
            {
                "type": "custom_tool_call",
                "call_id": "call_custom_pending",
                "name": "run_raw",
                "input": "raw custom input",
                "status": "in_progress"
            }
        ]))
        .expect("input item should deserialize");

        let messages = <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(input_items)
            .expect("input item should convert to universal");

        let roundtrip = universal_to_responses_input(&messages)
            .expect("universal messages should convert back to Responses input");
        assert_eq!(
            roundtrip[0].input_item_type,
            Some(openai::InputItemType::CustomToolCall)
        );
        assert_eq!(roundtrip[0].status, Some(openai::Status::InProgress));
        assert_eq!(roundtrip[0].input.as_deref(), Some("raw custom input"));
    }

    #[test]
    fn request_message_reasoning_accepts_array_objects() {
        let value = json!({
            "role": "assistant",
            "content": "The Greek name for the Sun is Helios.",
            "reasoning": [
                {
                    "id": "reasoning_1",
                    "content": "Identify Greek name for Sun."
                }
            ]
        });

        let parsed: ChatCompletionRequestMessageExt =
            serde_json::from_value(value).expect("message should deserialize");
        assert!(parsed.reasoning.is_some());

        let message = <Message as TryFromLLM<ChatCompletionRequestMessageExt>>::try_from(parsed)
            .expect("message should convert");
        match message {
            Message::Assistant { content, .. } => {
                let has_reasoning = match content {
                    AssistantContent::Array(parts) => parts
                        .iter()
                        .any(|part| matches!(part, AssistantContentPart::Reasoning { .. })),
                    AssistantContent::String(_) => false,
                };
                assert!(has_reasoning, "request reasoning should be preserved");
            }
            _ => panic!("expected assistant message"),
        }
    }

    #[cfg(feature = "anthropic")]
    #[test]
    fn chat_completions_cache_control_content_part_roundtrips_to_anthropic() {
        let value = json!({
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "Use the cached reference text.",
                    "cache_control": { "type": "ephemeral", "ttl": "1h" }
                },
                {
                    "type": "text",
                    "text": "Now summarize it."
                }
            ]
        });

        let parsed: ChatCompletionRequestMessageExt =
            serde_json::from_value(value).expect("message should deserialize");
        let message = <Message as TryFromLLM<ChatCompletionRequestMessageExt>>::try_from(parsed)
            .expect("message should convert");
        match &message {
            Message::User {
                content: UserContent::Array(parts),
            } => match &parts[0] {
                UserContentPart::Text(text) => assert!(text.cache_control.is_some()),
                other => panic!("expected text part, got {other:?}"),
            },
            other => panic!("expected user message with array content, got {other:?}"),
        }
        let anthropic_message =
            <crate::providers::anthropic::generated::InputMessage as TryFromLLM<Message>>::try_from(
                message,
            )
            .expect("message should convert to anthropic");

        let content = match anthropic_message.content {
            crate::providers::anthropic::generated::MessageContent::InputContentBlockArray(
                content,
            ) => content,
            other => panic!("expected anthropic content block array, got {other:?}"),
        };

        assert_eq!(content.len(), 2);
        assert_eq!(
            content[0]
                .cache_control
                .as_ref()
                .expect("cache_control should be preserved")
                .cache_control_ephemeral_type,
            crate::providers::anthropic::generated::CacheControlEphemeralType::Ephemeral
        );
        assert_eq!(
            content[0]
                .cache_control
                .as_ref()
                .expect("cache_control should be preserved")
                .ttl,
            Some(crate::providers::anthropic::generated::Ttl::The1H)
        );
        assert!(content[1].cache_control.is_none());
    }

    #[cfg(feature = "anthropic")]
    #[test]
    fn chat_completions_assistant_single_cache_control_part_roundtrips_to_anthropic() {
        let value = json!({
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "This assistant prefill should remain cacheable.",
                    "cache_control": { "type": "ephemeral", "ttl": "1h" }
                }
            ]
        });

        let parsed: ChatCompletionRequestMessageExt =
            serde_json::from_value(value).expect("message should deserialize");
        let message = <Message as TryFromLLM<ChatCompletionRequestMessageExt>>::try_from(parsed)
            .expect("message should convert");

        let Message::Assistant {
            content: AssistantContent::Array(parts),
            ..
        } = message
        else {
            panic!("expected assistant array content");
        };
        assert_eq!(parts.len(), 1);
        let AssistantContentPart::Text(text) = &parts[0] else {
            panic!("expected text part");
        };
        assert_eq!(text.text, "This assistant prefill should remain cacheable.");
        assert_eq!(
            text.cache_control
                .as_ref()
                .expect("cache_control should be preserved")
                .ttl,
            Some(crate::universal::CacheControlTtl::The1H)
        );

        let anthropic_message =
            <crate::providers::anthropic::generated::InputMessage as TryFromLLM<Message>>::try_from(
                Message::Assistant {
                    content: AssistantContent::Array(parts),
                    id: None,
                },
            )
            .expect("message should convert to anthropic");

        let content = match anthropic_message.content {
            crate::providers::anthropic::generated::MessageContent::InputContentBlockArray(
                content,
            ) => content,
            other => panic!("expected anthropic content block array, got {other:?}"),
        };
        assert_eq!(
            content[0]
                .cache_control
                .as_ref()
                .expect("cache_control should be preserved")
                .ttl,
            Some(crate::providers::anthropic::generated::Ttl::The1H)
        );
    }

    #[test]
    fn request_message_reasoning_signature_applies_to_tool_calls() {
        let value = json!({
            "role": "assistant",
            "content": "",
            "reasoning_signature": "thought_signature_123",
            "tool_calls": [{
                "id": "call_123",
                "type": "function",
                "function": {
                    "name": "get_summary",
                    "arguments": "{}"
                }
            }]
        });

        let parsed: ChatCompletionRequestMessageExt =
            serde_json::from_value(value).expect("message should deserialize");
        let message = <Message as TryFromLLM<ChatCompletionRequestMessageExt>>::try_from(parsed)
            .expect("message should convert");

        let Message::Assistant { content, .. } = message else {
            panic!("expected assistant message");
        };
        let AssistantContent::Array(parts) = content else {
            panic!("expected assistant array content");
        };
        let tool_call = parts
            .iter()
            .find_map(|part| match part {
                AssistantContentPart::ToolCall {
                    encrypted_content, ..
                } => Some(encrypted_content),
                _ => None,
            })
            .expect("tool call should be present");

        assert_eq!(tool_call.as_deref(), Some("thought_signature_123"));
    }

    #[test]
    fn response_message_tool_call_signature_becomes_reasoning_signature() {
        let message = Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                tool_call_id: "call_123".to_string(),
                tool_name: "list_collections".to_string(),
                arguments: ToolCallArguments::from(r#"{"database":"mydb"}"#.to_string()),
                encrypted_content: Some("dGhvdWdodF9zaWduYXR1cmVfMTIz".to_string()),
                provider_options: None,
                status: None,
                caller: None,
                provider_executed: None,
            }]),
            id: None,
        };

        let converted =
            <ChatCompletionResponseMessageExt as TryFromLLM<&Message>>::try_from(&message)
                .expect("message should convert");

        assert_eq!(
            converted.reasoning_signature.as_deref(),
            Some("dGhvdWdodF9zaWduYXR1cmVfMTIz")
        );
        assert_eq!(
            converted.base.tool_calls,
            Some(vec![openai::ToolCall {
                id: "call_123".to_string(),
                tool_call_type: openai::FluffyType::Function,
                function: Some(openai::PurpleFunction {
                    name: "list_collections".to_string(),
                    arguments: r#"{"database":"mydb"}"#.to_string(),
                }),
                custom: None,
            }])
        );
    }

    #[test]
    fn chat_messages_project_tool_discovery_history() {
        let messages = vec![
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::ToolDiscoveryCall {
                    tool_call_id: "call_tool_search_123".to_string(),
                    discovery_tool_name: "tool_search".to_string(),
                    query: None,
                    arguments: Some(json!({})),
                    status: Some("completed".to_string()),
                    execution: Some("server".to_string()),
                    provider_options: None,
                }]),
                id: None,
            },
            Message::Tool {
                content: vec![ToolContentPart::ToolDiscoveryResult(
                    crate::universal::ToolDiscoveryResultContentPart {
                        tool_call_id: "call_tool_search_123".to_string(),
                        discovery_tool_name: "tool_search".to_string(),
                        tools: vec![crate::universal::ToolDiscoveryResultItem {
                            tool_name: "search_code".to_string(),
                            tool: None,
                            provider_options: None,
                        }],
                        status: Some("completed".to_string()),
                        execution: Some("server".to_string()),
                        provider_options: None,
                    },
                )],
            },
        ];

        let converted = messages_to_chat_completion_messages(messages).unwrap();
        assert_eq!(converted.len(), 2);
        assert!(matches!(
            converted[0].role,
            openai::ChatCompletionRequestMessageRole::Assistant
        ));
        assert!(converted[0].content.is_none());
        assert_eq!(
            converted[0]
                .tool_calls
                .as_ref()
                .expect("assistant discovery call should become a tool call")[0]
                .id,
            "call_tool_search_123"
        );
        assert_eq!(
            converted[0]
                .tool_calls
                .as_ref()
                .expect("assistant discovery call should become a tool call")[0]
                .function
                .as_ref()
                .expect("tool call should be a function")
                .name,
            "tool_search"
        );
        assert!(matches!(
            converted[1].role,
            openai::ChatCompletionRequestMessageRole::Tool
        ));
        assert_eq!(
            converted[1].tool_call_id.as_deref(),
            Some("call_tool_search_123")
        );
        let Some(ChatCompletionRequestMessageContentExt::String(content)) =
            converted[1].content.as_ref()
        else {
            panic!("discovery result should become string tool content");
        };
        assert!(content.contains("tool_search"));
        assert!(content.contains("search_code"));
    }

    #[test]
    fn chat_messages_keep_non_discovery_assistant_content() {
        let messages = vec![Message::Assistant {
            content: AssistantContent::Array(vec![
                AssistantContentPart::Text(TextContentPart {
                    text: "done".to_string(),
                    cache_control: None,
                    encrypted_content: None,
                    provider_options: None,
                }),
                AssistantContentPart::ToolDiscoveryCall {
                    tool_call_id: "call_tool_search_123".to_string(),
                    discovery_tool_name: "tool_search".to_string(),
                    query: None,
                    arguments: Some(json!({})),
                    status: Some("completed".to_string()),
                    execution: Some("server".to_string()),
                    provider_options: None,
                },
            ]),
            id: None,
        }];

        let converted = messages_to_chat_completion_messages(messages).unwrap();
        assert_eq!(converted.len(), 1);
        assert!(matches!(
            converted[0].content,
            Some(ChatCompletionRequestMessageContentExt::String(ref text)) if text == "done"
        ));
        assert_eq!(
            converted[0]
                .tool_calls
                .as_ref()
                .expect("discovery call should become a tool call")[0]
                .id,
            "call_tool_search_123"
        );
    }

    #[test]
    fn response_message_rejects_distinct_tool_call_signatures() {
        let message = Message::Assistant {
            content: AssistantContent::Array(vec![
                AssistantContentPart::ToolCall {
                    tool_call_id: "call_1".to_string(),
                    tool_name: "first_tool".to_string(),
                    arguments: ToolCallArguments::from("{}".to_string()),
                    encrypted_content: Some("signature_one".to_string()),
                    provider_options: None,
                    status: None,
                    caller: None,
                    provider_executed: None,
                },
                AssistantContentPart::ToolCall {
                    tool_call_id: "call_2".to_string(),
                    tool_name: "second_tool".to_string(),
                    arguments: ToolCallArguments::from("{}".to_string()),
                    encrypted_content: Some("signature_two".to_string()),
                    provider_options: None,
                    status: None,
                    caller: None,
                    provider_executed: None,
                },
            ]),
            id: None,
        };

        let error = <ChatCompletionResponseMessageExt as TryFromLLM<&Message>>::try_from(&message)
            .expect_err("distinct per-call signatures should not be collapsed");

        assert!(error
            .to_string()
            .contains("multiple distinct encrypted tool/reasoning signatures"));
    }

    #[test]
    fn user_file_converts_to_responses_input_file() {
        let mut options = serde_json::Map::new();
        options.insert("title".into(), serde_json::Value::String("Doc".to_string()));

        let input = UserContentPart::File {
            data: serde_json::Value::String("Sample text.".to_string()),
            filename: None,
            media_type: "text/plain".to_string(),
            provider_options: Some(ProviderOptions { options }),
        };

        let converted = <openai::InputContent as TryFromLLM<UserContentPart>>::try_from(input)
            .expect("file should convert");

        assert_eq!(
            converted.input_content_type,
            openai::InputItemContentListType::InputFile
        );
        assert_eq!(converted.filename.as_deref(), Some("Doc"));
        assert_eq!(
            converted.file_data.as_deref(),
            Some("data:text/plain;base64,U2FtcGxlIHRleHQu")
        );
        assert!(converted.file_url.is_none());
    }

    #[test]
    fn user_file_is_not_supported_for_chat_completions() {
        let input = UserContentPart::File {
            data: serde_json::Value::String("Sample text.".to_string()),
            filename: None,
            media_type: "text/plain".to_string(),
            provider_options: None,
        };

        let error = convert_user_content_part_to_chat_completion_part(input)
            .expect_err("file should be rejected");

        assert!(matches!(error, ConvertError::UnsupportedInputType { .. }));
        assert!(error
            .to_string()
            .contains("is not supported by OpenAI ChatCompletions"));
    }

    #[test]
    fn user_file_uses_filename_when_present() {
        let input = UserContentPart::File {
            data: serde_json::Value::String("Sample text.".to_string()),
            filename: Some("explicit-name.txt".to_string()),
            media_type: "text/plain".to_string(),
            provider_options: None,
        };

        let converted = <openai::InputContent as TryFromLLM<UserContentPart>>::try_from(input)
            .expect("file should convert");

        assert_eq!(converted.filename.as_deref(), Some("explicit-name.txt"));
    }

    #[test]
    fn user_url_backed_file_does_not_synthesize_filename() {
        let input = UserContentPart::File {
            data: serde_json::Value::String("https://example.com/report.pdf".to_string()),
            filename: None,
            media_type: "application/pdf".to_string(),
            provider_options: None,
        };

        let converted = <openai::InputContent as TryFromLLM<UserContentPart>>::try_from(input)
            .expect("file should convert");

        assert_eq!(
            converted.input_content_type,
            openai::InputItemContentListType::InputFile
        );
        assert_eq!(
            converted.file_url.as_deref(),
            Some("https://example.com/report.pdf")
        );
        assert!(converted.filename.is_none());
        assert!(converted.file_data.is_none());
    }

    #[test]
    fn user_file_errors_for_non_string_data() {
        let input = UserContentPart::File {
            data: json!({ "not": "supported" }),
            filename: None,
            media_type: "application/pdf".to_string(),
            provider_options: None,
        };

        let error = <openai::InputContent as TryFromLLM<UserContentPart>>::try_from(input)
            .expect_err("object payload should fail");

        assert!(matches!(error, ConvertError::UnsupportedInputType { .. }));
        assert!(error
            .to_string()
            .contains("File data must be string-backed for OpenAI"));
    }

    #[test]
    fn chat_completions_user_file_errors_for_url_backed_file() {
        let input = UserContentPart::File {
            data: serde_json::Value::String("https://example.com/doc.txt".to_string()),
            filename: None,
            media_type: "text/plain".to_string(),
            provider_options: None,
        };

        let error = convert_user_content_part_to_chat_completion_part(input)
            .expect_err("url-backed file should fail");

        assert!(matches!(error, ConvertError::UnsupportedInputType { .. }));
        assert!(error
            .to_string()
            .contains("is not supported by OpenAI ChatCompletions"));
    }

    #[test]
    fn responses_input_file_imports_back_to_text_file() {
        let input = openai::InputContent {
            input_content_type: openai::InputItemContentListType::InputFile,
            file_data: Some("data:text/plain;base64,U2FtcGxlIHRleHQu".to_string()),
            filename: Some("Doc.txt".to_string()),
            ..Default::default()
        };

        let converted = <UserContentPart as TryFromLLM<openai::InputContent>>::try_from(input)
            .expect("file should import");

        match converted {
            UserContentPart::File {
                data,
                filename,
                media_type,
                ..
            } => {
                assert_eq!(data, serde_json::Value::String("Sample text.".to_string()));
                assert_eq!(filename.as_deref(), Some("Doc.txt"));
                assert_eq!(media_type, "text/plain");
            }
            other => panic!("expected file content, got {:?}", other),
        }
    }

    #[test]
    fn responses_input_file_url_infers_media_type_from_url() {
        let input = openai::InputContent {
            input_content_type: openai::InputItemContentListType::InputFile,
            file_url: Some("https://example.com/report.pdf".to_string()),
            filename: None,
            ..Default::default()
        };

        let converted = <UserContentPart as TryFromLLM<openai::InputContent>>::try_from(input)
            .expect("file should import");

        match converted {
            UserContentPart::File {
                data,
                filename,
                media_type,
                ..
            } => {
                assert_eq!(
                    data,
                    serde_json::Value::String("https://example.com/report.pdf".to_string())
                );
                assert!(filename.is_none());
                assert_eq!(media_type, "application/pdf");
            }
            other => panic!("expected file content, got {:?}", other),
        }
    }

    #[test]
    fn item_reference_inputs_are_skipped_during_conversion() {
        // When the AI SDK uses the stateful Responses API multi-turn format it sends
        // item_reference items (pointers to previous response objects) alongside
        // function_call_output items. item_reference has no `role`, so it previously
        // fell through to the `_ =>` wildcard which unconditionally required one.
        let inputs: Vec<openai::InputItem> = serde_json::from_value(json!([
            {
                "role": "developer",
                "content": "You are a helpful assistant."
            },
            {
                "role": "user",
                "content": [{ "type": "input_text", "text": "What is an eval?" }]
            },
            {
                "type": "item_reference",
                "id": "rs_0c44fb1b7c3aa2090069f3d9eb300481"
            },
            {
                "type": "item_reference",
                "id": "fc_0c44fb1b7c3aa2090069f3d9ecf80081"
            },
            {
                "type": "function_call_output",
                "call_id": "call_BjTKeTsoGfNs4klUOuTqeXxD",
                "output": "{\"result\": \"An eval is a structured test run.\"}"
            }
        ]))
        .expect("payload should deserialize");

        let messages = <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(inputs)
            .expect("item_reference items should be skipped, not cause a conversion error");

        let roles: Vec<&str> = messages
            .iter()
            .map(|m| match m {
                Message::Developer { .. } => "developer",
                Message::User { .. } => "user",
                Message::Tool { .. } => "tool",
                Message::Assistant { .. } => "assistant",
                Message::System { .. } => "system",
                Message::AdditionalTools { .. } => "additional_tools",
            })
            .collect();

        assert_eq!(
            roles,
            vec!["developer", "user", "tool"],
            "item_reference items should be silently dropped; function_call_output should become a tool message"
        );
    }

    #[test]
    fn responses_program_items_import_to_universal() {
        let messages = try_parse_responses_items_for_import(&json!([
            {
                "type": "program",
                "id": "prog_123",
                "call_id": "call_prog_123",
                "code": "text(JSON.stringify({ ok: true }));",
                "fingerprint": "opaque_state"
            },
            {
                "type": "program_output",
                "id": "prog_out_123",
                "call_id": "call_prog_123",
                "result": "{\"ok\":true}",
                "status": "completed"
            },
            {
                "type": "function_call",
                "id": "fc_123",
                "call_id": "call_inventory_123",
                "name": "get_inventory",
                "arguments": "{\"sku\":\"sku_123\"}",
                "caller": {
                    "type": "program",
                    "caller_id": "call_prog_123"
                }
            },
            {
                "type": "function_call_output",
                "call_id": "call_inventory_123",
                "output": "{\"sku\":\"sku_123\",\"available_units\":42}",
                "caller": {
                    "type": "program",
                    "caller_id": "call_prog_123"
                }
            }
        ]))
        .expect("program items should import");

        assert_eq!(messages.len(), 4);

        let Message::Assistant { content, id } = &messages[0] else {
            panic!("program should become assistant message");
        };
        assert_eq!(id.as_deref(), Some("prog_123"));
        let AssistantContent::Array(parts) = content else {
            panic!("program content should be array");
        };
        let AssistantContentPart::Program {
            call_id,
            code,
            fingerprint,
            id,
        } = &parts[0]
        else {
            panic!("expected program part");
        };
        assert_eq!(id.as_deref(), Some("prog_123"));
        assert_eq!(call_id, "call_prog_123");
        assert_eq!(code, "text(JSON.stringify({ ok: true }));");
        assert_eq!(fingerprint.as_deref(), Some("opaque_state"));

        let Message::Assistant { content, id } = &messages[1] else {
            panic!("program_output should become assistant message");
        };
        assert_eq!(id.as_deref(), Some("prog_out_123"));
        let AssistantContent::Array(parts) = content else {
            panic!("program_output content should be array");
        };
        let AssistantContentPart::ProgramOutput {
            call_id,
            result,
            status,
            id,
        } = &parts[0]
        else {
            panic!("expected program_output part");
        };
        assert_eq!(id.as_deref(), Some("prog_out_123"));
        assert_eq!(call_id, "call_prog_123");
        assert_eq!(result, "{\"ok\":true}");
        assert_eq!(status, "completed");

        let Message::Assistant { content, id } = &messages[2] else {
            panic!("function_call should become assistant message");
        };
        assert_eq!(id.as_deref(), Some("fc_123"));
        let AssistantContent::Array(parts) = content else {
            panic!("function_call content should be array");
        };
        let AssistantContentPart::ToolCall {
            tool_call_id,
            tool_name,
            caller,
            ..
        } = &parts[0]
        else {
            panic!("expected tool call part");
        };
        assert_eq!(tool_call_id, "call_inventory_123");
        assert_eq!(tool_name, "get_inventory");
        let caller = caller.as_ref().expect("caller should be preserved");
        assert_eq!(caller.caller_type, ToolCallerType::Program);
        assert_eq!(caller.caller_id.as_deref(), Some("call_prog_123"));

        let Message::Tool { content } = &messages[3] else {
            panic!("function_call_output should become tool message");
        };
        let ToolContentPart::ToolResult(tool_result) = &content[0] else {
            panic!("expected tool result part");
        };
        assert_eq!(tool_result.tool_call_id, "call_inventory_123");
        let caller = tool_result
            .caller
            .as_ref()
            .expect("tool result caller should be preserved");
        assert_eq!(caller.caller_type, ToolCallerType::Program);
        assert_eq!(caller.caller_id.as_deref(), Some("call_prog_123"));
    }

    #[test]
    fn responses_program_import_preserves_mixed_message_items() {
        let messages = try_parse_responses_items_for_import(&json!([
            {
                "type": "program",
                "id": "prog_123",
                "call_id": "call_prog_123",
                "code": "text('ready');"
            },
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "continue"
                    }
                ]
            }
        ]))
        .expect("mixed program and message items should import");

        assert_eq!(messages.len(), 2);

        let Message::Assistant { content, .. } = &messages[0] else {
            panic!("program should remain at the first position");
        };
        let AssistantContent::Array(parts) = content else {
            panic!("program content should be array");
        };
        assert!(
            matches!(parts.first(), Some(AssistantContentPart::Program { .. })),
            "first imported item should remain a program"
        );

        let Message::User { content } = &messages[1] else {
            panic!("normal message item should be preserved");
        };
        match content {
            UserContent::String(text) => assert_eq!(text, "continue"),
            UserContent::Array(parts) => {
                let Some(UserContentPart::Text(TextContentPart { text, .. })) = parts.first()
                else {
                    panic!("normal message item should import with text content");
                };
                assert_eq!(text, "continue");
            }
        }
    }

    #[test]
    fn responses_input_tool_search_output_reuses_null_call_id_from_previous_call() {
        let input_items: Vec<openai::InputItem> = serde_json::from_value(json!([
            {
                "type": "tool_search_call",
                "id": "tsc_1",
                "call_id": null,
                "status": "completed",
                "execution": "server",
                "arguments": {}
            },
            {
                "type": "tool_search_output",
                "id": "tso_1",
                "call_id": null,
                "status": "completed",
                "execution": "server",
                "tools": [{
                    "type": "function",
                    "name": "search_code",
                    "description": "Search code."
                }]
            }
        ]))
        .expect("input items should deserialize");

        let messages = <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(input_items)
            .expect("input items should convert");

        let Message::Assistant { content, .. } = &messages[0] else {
            panic!("tool_search_call should convert to assistant message");
        };
        let AssistantContent::Array(parts) = content else {
            panic!("assistant content should be array");
        };
        let AssistantContentPart::ToolDiscoveryCall {
            tool_call_id: call_id,
            ..
        } = &parts[0]
        else {
            panic!("first input item should become discovery call");
        };

        let Message::Tool { content } = &messages[1] else {
            panic!("tool_search_output should convert to tool message");
        };
        let ToolContentPart::ToolDiscoveryResult(result) = &content[0] else {
            panic!("second input item should become discovery result");
        };

        assert_eq!(call_id, "tsc_1");
        assert_eq!(result.tool_call_id, *call_id);
        assert_ne!(result.tool_call_id, "tso_1");
    }

    #[test]
    fn chat_completions_file_imports_back_to_text_file() {
        let input = openai::ChatCompletionRequestMessageContentPart {
            prompt_cache_breakpoint: None,
            text: None,
            chat_completion_request_message_content_part_type: openai::PurpleType::File,
            image_url: None,
            input_audio: None,
            file: Some(openai::File {
                file_data: Some("U2FtcGxlIHRleHQu".to_string()),
                file_id: None,
                filename: Some("Doc.txt".to_string()),
            }),
            refusal: None,
        };

        let converted = <UserContentPart as TryFromLLM<
            openai::ChatCompletionRequestMessageContentPart,
        >>::try_from(input)
        .expect("file should import");

        match converted {
            UserContentPart::File {
                data,
                filename,
                media_type,
                ..
            } => {
                assert_eq!(data, serde_json::Value::String("Sample text.".to_string()));
                assert_eq!(filename.as_deref(), Some("Doc.txt"));
                assert_eq!(media_type, "text/plain");
            }
            other => panic!("expected file content, got {:?}", other),
        }
    }

    // =========================================================================
    // RoleEnum → InputItemRole unsupported variant tests
    // =========================================================================

    #[test]
    fn output_item_with_critic_role_errors_on_input_conversion() {
        let output_item = openai::OutputItem {
            output_item_type: Some(openai::OutputItemType::Message),
            role: Some(openai::RoleEnum::Critic),
            content: Some(vec![openai::OutputMessageContent {
                output_message_content_type: openai::ContentType::OutputText,
                text: Some("review".to_string()),
                annotations: Some(vec![]),
                logprobs: None,
                refusal: None,
            }]),
            ..Default::default()
        };

        let result = <openai::InputItem as TryFromLLM<openai::OutputItem>>::try_from(output_item);
        assert!(
            result.is_err(),
            "Critic role should not silently coerce to User"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("RoleEnum::Critic") && err.contains("InputItemRole"),
            "Error should mention the unsupported role variant, got: {err}"
        );
    }

    #[test]
    fn output_item_with_discriminator_role_errors_on_input_conversion() {
        let output_item = openai::OutputItem {
            output_item_type: Some(openai::OutputItemType::Message),
            role: Some(openai::RoleEnum::Discriminator),
            ..Default::default()
        };

        let result = <openai::InputItem as TryFromLLM<openai::OutputItem>>::try_from(output_item);
        assert!(
            result.is_err(),
            "Discriminator role should not silently coerce"
        );
    }

    #[test]
    fn output_item_with_tool_role_errors_on_input_conversion() {
        let output_item = openai::OutputItem {
            output_item_type: Some(openai::OutputItemType::Message),
            role: Some(openai::RoleEnum::Tool),
            ..Default::default()
        };

        let result = <openai::InputItem as TryFromLLM<openai::OutputItem>>::try_from(output_item);
        assert!(result.is_err(), "Tool role should not silently coerce");
    }

    #[test]
    fn output_item_with_unknown_role_errors_on_input_conversion() {
        let output_item = openai::OutputItem {
            output_item_type: Some(openai::OutputItemType::Message),
            role: Some(openai::RoleEnum::Unknown),
            ..Default::default()
        };

        let result = <openai::InputItem as TryFromLLM<openai::OutputItem>>::try_from(output_item);
        assert!(result.is_err(), "Unknown role should not silently coerce");
    }

    // =========================================================================
    // AdditionalTools item type tests
    // =========================================================================

    fn additional_tools_function_tool() -> openai::InputItemTool {
        serde_json::from_value(crate::serde_json::json!({
            "type": "function",
            "name": "lookup_policy",
            "description": "Look up a policy",
            "parameters": {
                "type": "object",
                "properties": {
                    "topic": {"type": "string"}
                },
                "required": ["topic"],
                "additionalProperties": false
            },
            "strict": true
        }))
        .expect("valid input item tool")
    }

    #[test]
    fn additional_tools_input_item_converts_to_universal() {
        let input_item = openai::InputItem {
            id: Some("at_123".to_string()),
            input_item_type: Some(openai::InputItemType::AdditionalTools),
            role: Some(openai::InputItemRole::Developer),
            tools: Some(vec![additional_tools_function_tool()]),
            ..Default::default()
        };

        let messages =
            <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(vec![input_item]);
        let messages = messages.expect("AdditionalTools should convert to universal");
        assert_eq!(messages.len(), 1);
        let Message::AdditionalTools { tools, id } = &messages[0] else {
            panic!("expected AdditionalTools message");
        };
        assert_eq!(id.as_deref(), Some("at_123"));
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "lookup_policy");
        assert_eq!(tools[0].strict, Some(true));
    }

    #[test]
    fn additional_tools_output_item_converts_to_universal() {
        let output_tool = serde_json::to_value(additional_tools_function_tool())
            .and_then(serde_json::from_value)
            .expect("valid output item tool");
        let output_item = openai::OutputItem {
            id: Some("at_456".to_string()),
            output_item_type: Some(openai::OutputItemType::AdditionalTools),
            role: Some(openai::RoleEnum::Developer),
            tools: Some(vec![output_tool]),
            ..Default::default()
        };

        let messages =
            <Vec<Message> as TryFromLLM<Vec<openai::OutputItem>>>::try_from(vec![output_item]);
        let messages = messages.expect("AdditionalTools should convert to universal");
        assert_eq!(messages.len(), 1);
        let Message::AdditionalTools { tools, id } = &messages[0] else {
            panic!("expected AdditionalTools message");
        };
        assert_eq!(id.as_deref(), Some("at_456"));
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "lookup_policy");
    }

    #[test]
    fn additional_tools_universal_converts_to_responses_input_item() {
        let messages = <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(vec![
            openai::InputItem {
                id: Some("at_789".to_string()),
                input_item_type: Some(openai::InputItemType::AdditionalTools),
                role: Some(openai::InputItemRole::Developer),
                tools: Some(vec![additional_tools_function_tool()]),
                ..Default::default()
            },
        ])
        .expect("AdditionalTools should convert to universal");

        let input_items =
            universal_to_responses_input(&messages).expect("AdditionalTools should roundtrip");
        assert_eq!(input_items.len(), 1);
        let item = &input_items[0];
        assert_eq!(item.id.as_deref(), Some("at_789"));
        assert_eq!(
            item.input_item_type,
            Some(openai::InputItemType::AdditionalTools)
        );
        assert_eq!(item.role, Some(openai::InputItemRole::Developer));
        let tools = item.tools.as_ref().expect("tools should be present");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name.as_deref(), Some("lookup_policy"));
    }

    #[test]
    fn additional_tools_serialized_json_preserves_programmatic_tool_fields() {
        let mut tool = crate::universal::UniversalTool::function(
            "lookup_policy",
            Some("Look up a policy".to_string()),
            Some(crate::serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {"type": "string"}
                },
                "required": ["topic"],
                "additionalProperties": false
            })),
            Some(true),
        );
        tool.allowed_callers = Some(vec![
            crate::universal::tools::UniversalToolCaller::Programmatic,
        ]);
        tool.output_schema = Some(crate::serde_json::json!({
            "type": "object",
            "properties": {
                "policy": {"type": "string"}
            },
            "required": ["policy"],
            "additionalProperties": false
        }));

        let messages = vec![Message::AdditionalTools {
            tools: vec![tool],
            id: Some("at_programmatic".to_string()),
        }];
        let input_items =
            universal_to_responses_input(&messages).expect("AdditionalTools should convert");
        let values = responses_input_values_from_universal_context(&input_items, &messages)
            .expect("AdditionalTools should serialize");

        let serialized_tool = &values[0]["tools"][0];
        assert_eq!(serialized_tool["allowed_callers"], json!(["programmatic"]));
        assert_eq!(
            serialized_tool["output_schema"],
            json!({
                "type": "object",
                "properties": {
                    "policy": {"type": "string"}
                },
                "required": ["policy"],
                "additionalProperties": false
            })
        );

        let output_items =
            <Vec<openai::OutputItem> as TryFromLLM<Vec<Message>>>::try_from(messages.clone())
                .expect("AdditionalTools should convert to output items");
        let values = responses_output_values_from_universal_context(&output_items, &messages)
            .expect("AdditionalTools output should serialize");
        let serialized_tool = &values[0]["tools"][0];
        assert_eq!(serialized_tool["allowed_callers"], json!(["programmatic"]));
        assert_eq!(
            serialized_tool["output_schema"],
            json!({
                "type": "object",
                "properties": {
                    "policy": {"type": "string"}
                },
                "required": ["policy"],
                "additionalProperties": false
            })
        );
    }

    #[test]
    fn tool_discovery_serialized_json_preserves_programmatic_tool_fields() {
        let mut tool = crate::universal::UniversalTool::custom(
            "write_short_note",
            Some("Write a compact note".to_string()),
            Some(crate::serde_json::json!({"type": "text"})),
        );
        tool.allowed_callers = Some(vec![
            crate::universal::tools::UniversalToolCaller::Direct,
            crate::universal::tools::UniversalToolCaller::Programmatic,
        ]);
        tool.output_schema = Some(crate::serde_json::json!({
            "type": "object",
            "properties": {
                "note": {"type": "string"}
            },
            "required": ["note"],
            "additionalProperties": false
        }));

        let messages = vec![Message::Tool {
            content: vec![ToolContentPart::ToolDiscoveryResult(
                ToolDiscoveryResultContentPart {
                    tool_call_id: "call_tool_search_123".to_string(),
                    discovery_tool_name: "tool_search".to_string(),
                    tools: vec![ToolDiscoveryResultItem {
                        tool_name: "write_short_note".to_string(),
                        tool: Some(tool),
                        provider_options: None,
                    }],
                    status: Some("completed".to_string()),
                    execution: Some("client".to_string()),
                    provider_options: None,
                },
            )],
        }];

        let input_items =
            universal_to_responses_input(&messages).expect("tool discovery should convert");
        let values = responses_input_values_from_universal_context(&input_items, &messages)
            .expect("tool discovery should serialize");
        let serialized_tool = &values[0]["tools"][0];
        assert_eq!(
            serialized_tool["allowed_callers"],
            json!(["direct", "programmatic"])
        );
        assert_eq!(
            serialized_tool["output_schema"],
            json!({
                "type": "object",
                "properties": {
                    "note": {"type": "string"}
                },
                "required": ["note"],
                "additionalProperties": false
            })
        );

        let output_items =
            <Vec<openai::OutputItem> as TryFromLLM<Vec<Message>>>::try_from(messages.clone())
                .expect("tool discovery should convert to output items");
        let values = responses_output_values_from_universal_context(&output_items, &messages)
            .expect("tool discovery output should serialize");
        let serialized_tool = &values[0]["tools"][0];
        assert_eq!(
            serialized_tool["allowed_callers"],
            json!(["direct", "programmatic"])
        );
        assert_eq!(
            serialized_tool["output_schema"],
            json!({
                "type": "object",
                "properties": {
                    "note": {"type": "string"}
                },
                "required": ["note"],
                "additionalProperties": false
            })
        );
    }

    #[test]
    fn additional_tools_maps_between_output_and_input_item_types() {
        let output_item = openai::OutputItem {
            output_item_type: Some(openai::OutputItemType::AdditionalTools),
            role: Some(openai::RoleEnum::Developer),
            tools: Some(vec![]),
            ..Default::default()
        };

        let input_item =
            <openai::InputItem as TryFromLLM<openai::OutputItem>>::try_from(output_item)
                .expect("AdditionalTools should map between OutputItem and InputItem");
        assert_eq!(
            input_item.input_item_type,
            Some(openai::InputItemType::AdditionalTools)
        );
    }
}
