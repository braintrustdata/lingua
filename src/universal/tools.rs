use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Tool definition that can be called by the model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolDefinition {
    /// Unique identifier for this tool
    pub name: String,
    /// Human-readable description of what the tool does
    pub description: String,
    /// JSON schema for the tool's input parameters
    #[ts(type = "Record<string, any>")]
    pub input_schema: serde_json::Value,
    /// Whether this is a provider-native tool or client-managed
    pub execution_type: ToolExecutionType,
    /// Provider-specific configuration
    #[ts(optional)]
    #[ts(type = "Record<string, any>")]
    pub provider_config: Option<serde_json::Value>,
}

/// How a tool gets executed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ToolExecutionType {
    /// Client executes the tool (traditional function calling)
    ClientManaged,
    /// Provider executes the tool server-side
    ProviderManaged,
    /// Universal tool that maps to provider-specific implementations
    Universal,
}

/// A tool call made by the model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolCall {
    /// Unique ID for this tool call
    pub id: String,
    /// Name of the tool to call
    pub tool_name: String,
    /// Arguments to pass to the tool
    #[ts(type = "Record<string, any>")]
    pub arguments: serde_json::Value,
    /// Type of execution expected
    pub execution_type: ToolExecutionType,
}

/// Result from executing a tool
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolResult {
    /// ID of the tool call this result corresponds to
    pub call_id: String,
    /// Whether the tool execution succeeded
    pub success: bool,
    /// Tool output data
    #[ts(type = "Record<string, any>")]
    pub result: serde_json::Value,
    /// Error message if execution failed
    #[ts(optional)]
    pub error: Option<String>,
    /// Execution metadata (timing, cost, etc.)
    #[ts(optional)]
    #[ts(type = "Record<string, any>")]
    pub metadata: Option<serde_json::Value>,
}

/// Universal tool type that maps to provider-specific tools
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Tool {
    /// Web search capability
    WebSearch,
    /// Code execution environment
    CodeExecution,
    /// File/document processing
    FileProcessing,
    /// Image generation
    ImageGeneration,
    /// Audio processing
    AudioProcessing,
    /// Custom provider-specific tool
    ProviderSpecific(String),
}

/// Configuration for universal tools
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolConfig {
    /// The universal tool type
    pub tool: Tool,
    /// Configuration parameters
    #[ts(optional)]
    #[ts(type = "Record<string, any>")]
    pub config: Option<serde_json::Value>,
    /// Maximum number of times this tool can be used
    #[ts(optional)]
    pub max_uses: Option<u32>,
}

impl ToolDefinition {
    /// Create a simple tool definition
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
        execution_type: ToolExecutionType,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            execution_type,
            provider_config: None,
        }
    }

    /// Create a client-managed tool
    pub fn client_managed(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self::new(name, description, input_schema, ToolExecutionType::ClientManaged)
    }

    /// Create a provider-managed tool
    pub fn provider_managed(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self::new(name, description, input_schema, ToolExecutionType::ProviderManaged)
    }
}