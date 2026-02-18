// Canonical model configuration - change these to update all test cases
export const OPENAI_CHAT_COMPLETIONS_MODEL = "gpt-5-nano";
export const OPENAI_RESPONSES_MODEL = "gpt-5-nano";
// For parameters not supported by reasoning models (temperature, top_p, logprobs)
export const OPENAI_NON_REASONING_MODEL = "gpt-4o-mini";
export const ANTHROPIC_MODEL = "claude-sonnet-4-5-20250929";
// For Anthropic output_config.effort (requires Opus 4.5+)
export const ANTHROPIC_OPUS_MODEL = "claude-opus-4-6";
export const GOOGLE_MODEL = "gemini-2.5-flash";
export const GOOGLE_GEMINI_3_MODEL = "gemini-3-flash-preview";
export const GOOGLE_IMAGE_MODEL = "gemini-2.5-flash-image";
export const BEDROCK_MODEL = "us.anthropic.claude-haiku-4-5-20251001-v1:0";
export const BEDROCK_ANTHROPIC_MODEL =
  "us.anthropic.claude-haiku-4-5-20251001-v1:0";
