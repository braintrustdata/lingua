// Canonical model configuration - change these to update all test cases
export const OPENAI_CHAT_COMPLETIONS_MODEL = "gpt-4o-mini";
export const OPENAI_RESPONSES_MODEL = "gpt-5-nano";
// For parameters not supported by reasoning models (temperature, top_p, logprobs)
export const OPENAI_NON_REASONING_MODEL = "gpt-4o-mini";
export const ANTHROPIC_MODEL = "claude-sonnet-4-20250514";
// For Anthropic structured outputs (requires Sonnet 4.5+ for JSON schema output_format)
export const ANTHROPIC_STRUCTURED_OUTPUT_MODEL = "claude-sonnet-4-5-20250929";
export const GOOGLE_MODEL = "gemini-2.5-flash";
export const BEDROCK_ANTH_MODEL = "us.anthropic.claude-haiku-4-5-20251001-v1:0";
export const BEDROCK_CONVERSE_MODEL = "amazon.nova-micro-v1:0";
