/*!
Amazon Bedrock Converse API response types.

These types mirror the AWS Bedrock Converse API response structure with full serde support
for JSON serialization and compatibility with the Elmir format system.
*/

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Main Converse API response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct ConverseResponse {
    /// The model's response output
    pub output: BedrockConverseOutput,

    /// Reason why the model stopped generating
    pub stop_reason: BedrockStopReason,

    /// Token usage statistics
    pub usage: BedrockTokenUsage,

    /// Performance metrics for the request
    pub metrics: BedrockConverseMetrics,

    /// Optional trace information for debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<BedrockConverseTrace>,

    /// Additional model-specific response fields
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(skip)]
    pub additional_model_response_fields: Option<serde_json::Value>,
}

/// Converse output containing the generated message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockConverseOutput {
    /// The generated message
    pub message: BedrockOutputMessage,
}

/// Output message from the model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockOutputMessage {
    /// Role of the message (always assistant for responses)
    pub role: String,

    /// Content blocks in the message
    pub content: Vec<BedrockOutputContentBlock>,
}

/// Output content block types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
#[serde(tag = "type")]
pub enum BedrockOutputContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "toolUse")]
    ToolUse { tool_use: BedrockOutputToolUse },
}

/// Tool use in output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockOutputToolUse {
    /// Unique identifier for the tool use
    pub tool_use_id: String,

    /// Name of the tool
    pub name: String,

    /// Input parameters for the tool
    #[ts(type = "any")]
    pub input: serde_json::Value,
}

/// Reason why generation stopped
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
#[serde(rename_all = "snake_case")]
pub enum BedrockStopReason {
    /// Model reached a natural stopping point
    EndTurn,
    /// Maximum token limit reached
    MaxTokens,
    /// Stop sequence encountered
    StopSequence,
    /// Tool use requested
    ToolUse,
    /// Content filtered by guardrails
    ContentFiltered,
}

/// Token usage statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockTokenUsage {
    /// Number of input tokens
    pub input_tokens: i32,

    /// Number of output tokens
    pub output_tokens: i32,

    /// Total number of tokens
    pub total_tokens: i32,
}

/// Performance metrics for the request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockConverseMetrics {
    /// Latency in milliseconds
    pub latency_ms: i64,
}

/// Streaming Converse API response event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
#[serde(tag = "type")]
pub enum ConverseStreamResponse {
    /// Start of a message
    #[serde(rename = "messageStart")]
    MessageStart {
        /// Event details
        role: String,
    },

    /// Start of a content block
    #[serde(rename = "contentBlockStart")]
    ContentBlockStart {
        /// Index of the content block
        content_block_index: i32,
        /// Start event details
        start: BedrockContentBlockStart,
    },

    /// Content block delta (incremental content)
    #[serde(rename = "contentBlockDelta")]
    ContentBlockDelta {
        /// Index of the content block being updated
        content_block_index: i32,
        /// The incremental content delta
        delta: BedrockContentBlockDelta,
    },

    /// End of a content block
    #[serde(rename = "contentBlockStop")]
    ContentBlockStop {
        /// Index of the content block
        content_block_index: i32,
    },

    /// End of a message
    #[serde(rename = "messageStop")]
    MessageStop {
        /// Reason why generation stopped
        stop_reason: BedrockStopReason,
        /// Additional model-specific fields
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(skip)]
        additional_model_response_fields: Option<serde_json::Value>,
    },

    /// Metadata about the stream
    #[serde(rename = "metadata")]
    Metadata {
        /// Token usage statistics
        usage: BedrockTokenUsage,
        /// Performance metrics
        metrics: BedrockConverseMetrics,
    },
}

/// Content block start event details
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
#[serde(tag = "type")]
pub enum BedrockContentBlockStart {
    #[serde(rename = "toolUse")]
    ToolUse { tool_use_id: String, name: String },
}

/// Content block delta with incremental content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
#[serde(tag = "type")]
pub enum BedrockContentBlockDelta {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "toolUse")]
    ToolUse { input: String },
}

/// Trace information for debugging and monitoring
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockConverseTrace {
    /// Unique identifier for the trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// Guardrail trace information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrail: Option<BedrockGuardrailTrace>,
}

/// Guardrail trace information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockGuardrailTrace {
    /// Action taken by the guardrail
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    /// Input assessments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_assessments: Option<Vec<BedrockGuardrailAssessment>>,

    /// Output assessments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_assessments: Option<Vec<BedrockGuardrailAssessment>>,
}

/// Individual guardrail assessment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockGuardrailAssessment {
    /// Content policy assessment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_policy: Option<BedrockContentPolicyAssessment>,

    /// Sensitive information policy assessment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive_information_policy: Option<BedrockSensitiveInformationPolicyAssessment>,

    /// Topic policy assessment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_policy: Option<BedrockTopicPolicyAssessment>,

    /// Word policy assessment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_policy: Option<BedrockWordPolicyAssessment>,
}

/// Content policy assessment details
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockContentPolicyAssessment {
    /// Filters that were triggered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<BedrockContentFilter>>,
}

/// Content filter details
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockContentFilter {
    /// Filter type
    #[serde(rename = "type")]
    pub filter_type: String,

    /// Confidence level
    pub confidence: String,

    /// Action taken
    pub action: String,
}

/// Sensitive information policy assessment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockSensitiveInformationPolicyAssessment {
    /// PII entities detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pii_entities: Option<Vec<BedrockPIIEntity>>,

    /// Regexes that matched
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regexes: Option<Vec<BedrockRegexMatch>>,
}

/// PII entity details
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockPIIEntity {
    /// Entity type
    #[serde(rename = "type")]
    pub entity_type: String,

    /// Match text
    #[serde(rename = "match")]
    pub match_text: String,

    /// Action taken
    pub action: String,
}

/// Regex match details
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockRegexMatch {
    /// Regex name
    pub name: String,

    /// Match text
    #[serde(rename = "match")]
    pub match_text: String,

    /// Action taken
    pub action: String,
}

/// Topic policy assessment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockTopicPolicyAssessment {
    /// Topics that were identified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<BedrockTopic>>,
}

/// Topic details
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockTopic {
    /// Topic name
    pub name: String,

    /// Topic type
    #[serde(rename = "type")]
    pub topic_type: String,

    /// Action taken
    pub action: String,
}

/// Word policy assessment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockWordPolicyAssessment {
    /// Custom words that were detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_words: Option<Vec<BedrockCustomWord>>,

    /// Managed word lists that were triggered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_word_lists: Option<Vec<BedrockManagedWordList>>,
}

/// Custom word details
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockCustomWord {
    /// Match text
    #[serde(rename = "match")]
    pub match_text: String,

    /// Action taken
    pub action: String,
}

/// Managed word list details
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockManagedWordList {
    /// Match text
    #[serde(rename = "match")]
    pub match_text: String,

    /// Word list type
    #[serde(rename = "type")]
    pub list_type: String,

    /// Action taken
    pub action: String,
}
