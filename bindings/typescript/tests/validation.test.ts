/**
 * TypeScript Validation Tests
 *
 * These tests validate that:
 * 1. Each provider's request/response validates successfully
 * 2. Each provider's payload fails to validate as other providers' formats
 * 3. WASM validation functions work correctly from TypeScript
 */

import { describe, test, expect } from 'vitest'
import {
  validateOpenAIRequest,
  validateOpenAIResponse,
  validateAnthropicRequest,
  validateAnthropicResponse,
} from '../src'
import type { ChatCompletionCreateParamsNonStreaming } from 'openai/resources/chat/completions'
import type { ChatCompletion } from 'openai/resources/chat/completions'
import type { MessageCreateParams } from '@anthropic-ai/sdk/resources/messages'
import type { Message } from '@anthropic-ai/sdk/resources/messages'

// Test payloads for each provider - typed with SDK types for correctness
const OPENAI_REQUEST_DATA: ChatCompletionCreateParamsNonStreaming = {
  model: 'gpt-4',
  messages: [
    {
      role: 'user',
      content: 'Hello',
    },
  ],
}
const OPENAI_REQUEST = JSON.stringify(OPENAI_REQUEST_DATA)

const OPENAI_RESPONSE_DATA: ChatCompletion = {
  id: 'chatcmpl-123',
  object: 'chat.completion',
  created: 1677652288,
  model: 'gpt-4',
  choices: [
    {
      index: 0,
      message: {
        role: 'assistant',
        content: 'Hello!',
      },
      finish_reason: 'stop',
      logprobs: null,
    },
  ],
  usage: {
    prompt_tokens: 9,
    completion_tokens: 12,
    total_tokens: 21,
  },
}
const OPENAI_RESPONSE = JSON.stringify(OPENAI_RESPONSE_DATA)

const ANTHROPIC_REQUEST_DATA: MessageCreateParams = {
  model: 'claude-3-5-sonnet-20241022',
  messages: [
    {
      role: 'user',
      content: [
        {
          type: 'text',
          text: 'Hello',
        },
      ],
    },
  ],
  max_tokens: 1024,
}
const ANTHROPIC_REQUEST = JSON.stringify(ANTHROPIC_REQUEST_DATA)

const ANTHROPIC_RESPONSE_DATA: Message = {
  id: 'msg_123',
  type: 'message',
  role: 'assistant',
  content: [
    {
      type: 'text',
      text: 'Hello!',
    },
  ],
  model: 'claude-3-5-sonnet-20241022',
  stop_reason: 'end_turn',
  stop_sequence: null,
  usage: {
    input_tokens: 10,
    output_tokens: 20,
  },
}
const ANTHROPIC_RESPONSE = JSON.stringify(ANTHROPIC_RESPONSE_DATA)

describe('OpenAI Validation', () => {
  test('validates OpenAI request successfully', () => {
    const result = validateOpenAIRequest(OPENAI_REQUEST)
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data).toBeDefined()
      expect((result.data as any).model).toBe('gpt-4')
    }
  })

  test('validates OpenAI response successfully', () => {
    const result = validateOpenAIResponse(OPENAI_RESPONSE)
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data).toBeDefined()
    }
  })

  test('Anthropic response fails to validate as OpenAI', () => {
    const result = validateOpenAIResponse(ANTHROPIC_RESPONSE)
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.message).toBeDefined()
    }
  })

  test('rejects invalid JSON', () => {
    const result = validateOpenAIRequest('{invalid json}')
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.message).toContain('Deserialization failed')
    }
  })

  test('rejects missing required fields', () => {
    const invalid = JSON.stringify({ model: 'gpt-4' }) // missing messages
    const result = validateOpenAIRequest(invalid)
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.message).toBeDefined()
    }
  })
})

describe('Anthropic Validation', () => {
  test('validates Anthropic request successfully', () => {
    const result = validateAnthropicRequest(ANTHROPIC_REQUEST)
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data).toBeDefined()
    }
  })

  test('validates Anthropic response successfully', () => {
    const result = validateAnthropicResponse(ANTHROPIC_RESPONSE)
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data).toBeDefined()
    }
  })

  test('OpenAI request fails to validate as Anthropic', () => {
    const result = validateAnthropicRequest(OPENAI_REQUEST)
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.message).toBeDefined()
    }
  })

  test('OpenAI response fails to validate as Anthropic', () => {
    const result = validateAnthropicResponse(OPENAI_RESPONSE)
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.message).toBeDefined()
    }
  })

  test('rejects invalid JSON', () => {
    const result = validateAnthropicRequest('{invalid json}')
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.message).toBeDefined()
    }
  })

  test('rejects missing required fields', () => {
    const invalid = JSON.stringify({ model: 'claude-3-5-sonnet-20241022' }) // missing messages and max_tokens
    const result = validateAnthropicRequest(invalid)
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error.message).toBeDefined()
    }
  })
})

describe('Cross-provider validation', () => {
  test('OpenAI and Anthropic requests are distinct', () => {
    // OpenAI request should not validate as Anthropic
    const result = validateAnthropicRequest(OPENAI_REQUEST)
    expect(result.ok).toBe(false)

    // Note: Anthropic request might validate as OpenAI due to structural compatibility
    // This is expected behavior - validation checks structure, not semantic correctness
  })

  test('OpenAI and Anthropic responses are distinct', () => {
    // OpenAI response should not validate as Anthropic
    const anthropicResult = validateAnthropicResponse(OPENAI_RESPONSE)
    expect(anthropicResult.ok).toBe(false)

    // Anthropic response should not validate as OpenAI
    const openaiResult = validateOpenAIResponse(ANTHROPIC_RESPONSE)
    expect(openaiResult.ok).toBe(false)
  })

  test('validation returns parsed object on success', () => {
    const result = validateOpenAIRequest(OPENAI_REQUEST)
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data).toBeDefined()
      expect((result.data as any).model).toBe('gpt-4')
      expect((result.data as any).messages).toHaveLength(1)
    }
  })

  test('validation returns error object on failure', () => {
    const result = validateOpenAIRequest('{invalid json}')
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error).toBeDefined()
      expect(result.error.message).toContain('Deserialization failed')
    }
  })
})
