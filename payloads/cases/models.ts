// Canonical model configuration - change these to update all test cases
export const OPENAI_CHAT_COMPLETIONS_MODEL = "gpt-5-nano";
export const OPENAI_RESPONSES_MODEL = "gpt-5-nano";
export const OPENAI_REASONING_NONE_MODEL = "gpt-5.2";
// Mini reasoning model: supports reasoning_effort but requires /v1/responses when combined with function tools
export const OPENAI_MINI_REASONING_MODEL = "gpt-5.4-mini";
// For parameters not supported by reasoning models (temperature, top_p, logprobs)
export const OPENAI_NON_REASONING_MODEL = "gpt-4o-mini";
export const ANTHROPIC_MODEL = "claude-sonnet-4-5-20250929";
export const ANTHROPIC_FABLE_MODEL = "claude-fable-5";
// For Anthropic output_config.effort (requires Opus 4.5+)
export const ANTHROPIC_OPUS_MODEL = "claude-opus-4-6";
// For Anthropic mid-conversation system messages (requires Opus 4.8+)
export const ANTHROPIC_OPUS_4_8_MODEL = "claude-opus-4-8";
export const GOOGLE_MODEL = "gemini-3.5-flash";
export const GOOGLE_GEMINI_3_MODEL = "gemini-3-flash-preview";
export const GOOGLE_IMAGE_MODEL = "gemini-2.5-flash-image";
export const GOOGLE_TTS_MODEL = "gemini-2.5-flash-preview-tts";
export const BEDROCK_MODEL = "us.anthropic.claude-haiku-4-5-20251001-v1:0";
export const BEDROCK_ANTHROPIC_MODEL =
  "us.anthropic.claude-haiku-4-5-20251001-v1:0";
export const VERTEX_ANTHROPIC_MODEL =
  "publishers/anthropic/models/claude-haiku-4-5";

// Baseten serves OSS models behind an OpenAI-compatible (chat-completions) API.
// Used to capture real OSS-provider wire behavior (e.g. GLM streaming framing)
// that native OpenAI does not exhibit.
export const BASETEN_BASE_URL = "https://inference.baseten.co/v1";
export const BASETEN_MODEL = "zai-org/GLM-5.2";
