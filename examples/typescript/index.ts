import {
  type Message,
  linguaToChatCompletionsMessages,
  linguaToAnthropicMessages,
  // TODO: Need singular versions of these
  chatCompletionsMessagesToLingua,
  anthropicMessagesToLingua,
} from '@braintrust/lingua'

import OpenAI from 'openai'
import Anthropic from '@anthropic-ai/sdk'

async function basicUsage() {
  // Write messages and tools in Lingua's universal format
  const messages: Message[] = [
    {
      role: 'user',
      content: 'Tell me a fun fact about pizza',
    },
  ]

  // (Imagine we have a feature flag controlling which model we use)
  const useOpenAi = Math.random() > 0.5

  // Call any provider
  const response = useOpenAi
    ? chatCompletionsMessagesToLingua(await createOpenAiCompletion(messages))
    : anthropicMessagesToLingua(await createAnthropicCompletion(messages))

  // ✨ Proceed in Lingua format ✨
  return response
}

async function main() {
  const hasOpenAiApiKey = !!process.env.OPENAI_API_KEY
  const hasAnthropicApiKey = !!process.env.ANTHROPIC_API_KEY

  if (hasOpenAiApiKey && hasAnthropicApiKey) {
    console.log('Getting a fun fact about pizza...')

    const response = await basicUsage()

    console.log('Response:')
    console.log(JSON.stringify(response, null, 2))
  } else {
    console.log(
      'Skipping basic usage example - both OpenAI and Anthropic API keys are required',
    )
  }
}

const createOpenAiCompletion = async (messages: Message[]) => {
  const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY })
  const openaiMessages =
    linguaToChatCompletionsMessages<OpenAI.Chat.ChatCompletionMessageParam[]>(
      messages,
    )
  const openAiResponse = await openai.chat.completions.create({
    model: 'gpt-5-nano',
    messages: openaiMessages,
  })

  return openAiResponse.choices[0].message
}

const createAnthropicCompletion = async (messages: Message[]) => {
  const anthropic = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY })
  const anthropicMessages =
    linguaToAnthropicMessages<Anthropic.MessageParam[]>(messages)
  const anthropicResponse = await anthropic.messages.create({
    model: 'claude-haiku-4-5-20251001',
    messages: anthropicMessages,
    max_tokens: 100,
  })

  return anthropicResponse
}

async function example() {
  // Write your conversation in Lingua's universal format
  const messages: Message[] = [
    {
      role: 'user',
      content: 'Tell me a fun fact about pizza',
    },
  ]

  // Now imagine we have a feature flag controlling which model we use
  const useOpenAi = Math.random() > 0.5

  let linguaResponse: Message[]
  if (useOpenAi) {
    const openAiResponse = await createOpenAiCompletion(messages)

    linguaResponse = chatCompletionsMessagesToLingua([openAiResponse])
  } else {
    const anthropicResponse = await createAnthropicCompletion(messages)

    linguaResponse = anthropicMessagesToLingua([anthropicResponse])
  }

  // ✨ Proceed in Lingua format ✨
  return linguaResponse
}

/**
 * Test ideas:
 * - Agent loop
 * - Fallback to different provider within agent loop
 * - Fan out to multiple providers using same lingua messages, then do something cool with the results (choose best candidate perhaps or have LLM choose best?)
 */

main().catch(console.error)
