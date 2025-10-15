import { describe, test, expect, beforeAll } from 'vitest'
import OpenAI from 'openai'
import Anthropic from '@anthropic-ai/sdk'
import type { Message } from '../../src'
import {
  linguaToChatCompletionsMessages,
  linguaToAnthropicMessages,
  linguaToResponsesMessages,
} from '../../src'

// Required environment variables for this test suite
const REQUIRED_ENV_VARS = ['OPENAI_API_KEY', 'ANTHROPIC_API_KEY'] as const

for (const key of REQUIRED_ENV_VARS) {
  if (!process.env[key]) {
    throw new Error(`Missing required environment variable: ${key}`)
  }
}

const CHEAP_TEXT_COMPLETION_MODELS = {
  openai: 'gpt-4o-mini',
  anthropic: 'claude-3-5-haiku-20241022',
  responses: 'gpt-4o-mini',
}

// Initialize clients
const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY })
const anthropic = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY })

// TODO: Convert each response to lingua, then we can pass to a shared function to format output
// Maybe try building a mini provider-agnostic AI SDK in this test or a different test
describe('Write once, call any provider', () => {
  describe('Simple text, single turn', () => {
    test('OpenAI Chat Completions API', async () => {
      const conversation: Message[] = [{ role: 'user', content: 'Hi' }]

      // FIXME: This returns `unknown[]`
      const messages = linguaToChatCompletionsMessages(conversation)

      const response = await openai.chat.completions.create({
        messages: messages as any,
        model: CHEAP_TEXT_COMPLETION_MODELS.openai,
        max_tokens: 2,
      })

      expect(response).toBeDefined()
      expect(response.choices).toHaveLength(1)
      expect(response.choices[0].message).toBeDefined()
      expect(response.choices[0].message.role).toBe('assistant')
      expect(response.choices[0].message.content).toBeDefined()
      expect(response.choices[0].finish_reason).toBeDefined()
      expect(response.usage).toBeDefined()
      expect(response.usage?.total_tokens).toBeGreaterThan(0)

      console.log(
        'OpenAI response content: ',
        `"${response.choices[0].message.content}"`,
      )
    })

    test('Anthropic Messages API', async () => {
      const conversation: Message[] = [{ role: 'user', content: 'Hi' }]

      // FIXME: This returns `unknown[]`
      const messages = linguaToAnthropicMessages(conversation)

      const response = await anthropic.messages.create({
        messages: messages as any,
        model: CHEAP_TEXT_COMPLETION_MODELS.anthropic,
        max_tokens: 2,
      })

      expect(response).toBeDefined()
      expect(response.role).toBe('assistant')
      expect(response.content).toBeDefined()
      expect(Array.isArray(response.content)).toBe(true)
      expect(response.content.length).toBeGreaterThan(0)
      expect(response.usage).toBeDefined()
      expect(response.usage.input_tokens).toBeGreaterThan(0)

      for (const block of response.content) {
        if (block.type === 'text') {
          console.log('Anthropic response content: ', `"${block.text}"`)
        }
      }
    })

    test('OpenAI Responses API', async () => {
      const conversation: Message[] = [{ role: 'user', content: 'Hi' }]

      // FIXME: This returns `unknown[]`
      const input = linguaToResponsesMessages(conversation)

      const response = await openai.responses.create({
        input: input as any,
        model: CHEAP_TEXT_COMPLETION_MODELS.responses,
        max_output_tokens: 16,
      })

      expect(response).toBeDefined()
      expect(response.output).toBeDefined()
      expect(response.usage).toBeDefined()

      for (const item of response.output) {
        if (item.type === 'message') {
          for (const block of item.content) {
            if (block.type === 'output_text') {
              console.log('OpenAI response text content: ', `"${block.text}"`)
            }
          }
        }
      }
    })
  })

  describe.todo(
    'Multi-turn conversation - same Lingua code, multiple APIs',
    () => {
      // For each supported provider, make a simple text completion, then convert response to lingua and push to messages, then make another completion with updated messages
    },
  )
})
