use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Token usage information with normalized counting across providers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TokenUsage {
    /// Input tokens consumed (normalized across providers)
    pub input_tokens: u64,
    /// Output tokens generated
    pub output_tokens: u64,
    /// Total tokens (input + output)
    pub total_tokens: u64,
    /// Reasoning tokens (for models with visible reasoning)
    #[ts(optional)]
    pub reasoning_tokens: Option<u64>,
    /// Cached tokens (tokens served from cache)
    #[ts(optional)]
    pub cached_tokens: Option<u64>,
    /// Provider-specific token breakdown
    #[ts(optional)]
    #[ts(type = "Record<string, any>")]
    pub provider_breakdown: Option<serde_json::Value>,
}

/// Cost information for the request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CostInfo {
    /// Total cost in USD
    pub total_cost: f64,
    /// Input cost breakdown
    pub input_cost: f64,
    /// Output cost breakdown  
    pub output_cost: f64,
    /// Cost for cached tokens (usually discounted)
    #[ts(optional)]
    pub cache_cost: Option<f64>,
    /// Cost for reasoning tokens
    #[ts(optional)]
    pub reasoning_cost: Option<f64>,
    /// Currency (default: "USD")
    pub currency: String,
}

/// Complete usage information combining tokens and cost
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Usage {
    /// Token usage breakdown
    pub tokens: TokenUsage,
    /// Cost information
    #[ts(optional)]
    pub cost: Option<CostInfo>,
    /// Provider that generated this usage info
    pub provider: String,
    /// Model used for this request
    pub model: String,
}

impl TokenUsage {
    /// Create a simple token usage record
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            reasoning_tokens: None,
            cached_tokens: None,
            provider_breakdown: None,
        }
    }
}

impl CostInfo {
    /// Create cost info in USD
    pub fn usd(input_cost: f64, output_cost: f64) -> Self {
        Self {
            total_cost: input_cost + output_cost,
            input_cost,
            output_cost,
            cache_cost: None,
            reasoning_cost: None,
            currency: "USD".to_string(),
        }
    }
}