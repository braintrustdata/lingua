/**
 * TypeScript helper functions for creating Lingua tools
 */

import type { Tool } from "./generated/Tool";

/**
 * Create a client-defined function tool.
 *
 * Client tools are executed by your application code, not by the provider.
 * When the model calls a client tool:
 * 1. You receive a tool call with the function name and arguments
 * 2. Your application executes the function
 * 3. You return the result to the model
 *
 * @example
 * ```typescript
 * const tool = clientTool({
 *   name: "get_weather",
 *   description: "Get current weather for a location",
 *   input_schema: {
 *     type: "object",
 *     properties: {
 *       location: { type: "string" },
 *       unit: { type: "string", enum: ["celsius", "fahrenheit"] }
 *     },
 *     required: ["location"]
 *   }
 * });
 * ```
 */
export function clientTool(params: {
  name: string;
  description: string;
  input_schema: Record<string, any>;
  provider_options?: Record<string, any>;
}): Tool {
  return {
    type: "function",
    name: params.name,
    description: params.description,
    input_schema: params.input_schema,
    provider_options: params.provider_options,
  };
}

/**
 * Create a provider-native tool.
 *
 * Provider tools are executed by the LLM provider's infrastructure, not your code.
 * These tools may have additional costs and are provider-specific.
 *
 * @param tool_type - Provider-specific tool type identifier (e.g., "web_search_20250305")
 * @param options - Optional configuration
 *
 * @example
 * ```typescript
 * const tool = providerTool("web_search_20250305", {
 *   config: {
 *     max_uses: 5,
 *     allowed_domains: ["wikipedia.org"]
 *   }
 * });
 * ```
 */
export function providerTool(
  tool_type: string,
  options?: {
    name?: string;
    config?: Record<string, any>;
  }
): Tool {
  return {
    type: "provider",
    tool_type,
    name: options?.name ?? null,
    config: options?.config,
  };
}

/**
 * Convenience helpers for common provider tools.
 *
 * These helpers provide type-safe configuration for known provider tools,
 * but you can always use providerTool() directly for new or custom tools.
 */
export const ProviderTools = {
  anthropic: {
    /**
     * Anthropic web search tool (requires anthropic-beta: web-search-2025-03-05)
     * Cost: $10 per 1,000 searches
     */
    webSearch: (config?: {
      max_uses?: number;
      allowed_domains?: string[];
      blocked_domains?: string[];
      user_location?: {
        city?: string;
        region?: string;
        country?: string;
        timezone?: string;
      };
    }): Tool => providerTool("web_search_20250305", { config }),

    /**
     * Anthropic bash shell tool
     */
    bash: (config?: { max_uses?: number }): Tool =>
      providerTool("bash_20250124", { config }),

    /**
     * Anthropic text editor tool (2025-01-24 version)
     */
    textEditor_20250124: (config?: { max_characters?: number }): Tool =>
      providerTool("text_editor_20250124", { config }),

    /**
     * Anthropic text editor tool (2025-04-29 version)
     */
    textEditor_20250429: (config?: { max_characters?: number }): Tool =>
      providerTool("text_editor_20250429", { config }),

    /**
     * Anthropic text editor tool (2025-07-28 version)
     */
    textEditor_20250728: (config?: { max_characters?: number }): Tool =>
      providerTool("text_editor_20250728", { config }),
  },

  openai: {
    /**
     * OpenAI computer use tool
     */
    computer: (config?: {
      display_width_px?: number;
      display_height_px?: number;
      environment?: any;
    }): Tool => providerTool("computer_use_preview", { config }),

    /**
     * OpenAI code interpreter tool
     */
    codeInterpreter: (config?: { container?: any }): Tool =>
      providerTool("code_interpreter", { config }),

    /**
     * OpenAI web search tool
     */
    webSearch: (config?: {
      search_context_size?: "low" | "medium" | "high";
      user_location?: any;
    }): Tool => providerTool("web_search", { config }),

    /**
     * OpenAI web search tool (2025-08-26 version)
     */
    webSearch_2025_08_26: (config?: {
      search_context_size?: "low" | "medium" | "high";
      user_location?: any;
    }): Tool => providerTool("web_search_2025_08_26", { config }),
  },
};
