import {
  type Message,
  type Tool,
  linguaToChatCompletionsMessages,
  linguaToAnthropicMessages,
  chatCompletionsMessagesToLingua,
  anthropicMessagesToLingua,
  linguaToolsToOpenAI,
  linguaToolsToAnthropic,
  clientTool,
} from "@braintrust/lingua";

import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";
import { centerText } from "./center-text";

async function basicUsage() {
  // Write messages and tools in Lingua's universal format
  const messages: Message[] = [
    {
      role: "user",
      content: "Tell me a little-known fact about pizza",
    },
  ];

  console.log("\n📝 Step 1: Write in Lingua's universal format");
  console.log("   Message:", JSON.stringify(messages[0].content));

  // (Imagine we have a feature flag controlling which model we use)
  const useOpenAi = Math.random() > 0.5;
  const provider = useOpenAi ? "OpenAI" : "Anthropic";

  console.log(`\n🎲 Step 2: Dynamically choosing provider: ${provider}`);
  console.log("\n🔄 Step 3: Calling provider API...");

  // Call any provider
  const response = useOpenAi
    ? chatCompletionsMessagesToLingua(await createOpenAiCompletion(messages))
    : anthropicMessagesToLingua(await createAnthropicCompletion(messages));

  console.log("\n✅ Step 4: Response converted back to Lingua:");

  const content = response[0].content;
  console.log(
    typeof content === "string" ? content : JSON.stringify(content, null, 2)
  );

  // ✨ Proceed in Lingua format ✨
  return response;
}

async function main() {
  const hasOpenAiApiKey = !!process.env.OPENAI_API_KEY;
  const hasAnthropicApiKey = !!process.env.ANTHROPIC_API_KEY;

  if (hasOpenAiApiKey && hasAnthropicApiKey) {
    await basicUsage();
    await wordleSolverAgent();
  } else {
    console.log(
      "⚠️  Skipping example - both OPENAI_API_KEY and ANTHROPIC_API_KEY required"
    );
  }
}

const createOpenAiCompletion = async (messages: Message[]) => {
  const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
  const openaiMessages =
    linguaToChatCompletionsMessages<OpenAI.Chat.ChatCompletionMessageParam[]>(
      messages
    );
  const openAiResponse = await openai.chat.completions.create({
    model: "gpt-5-nano",
    messages: openaiMessages,
  });

  return [openAiResponse.choices[0].message];
};

const createAnthropicCompletion = async (messages: Message[]) => {
  const anthropic = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY });
  const anthropicMessages =
    linguaToAnthropicMessages<Anthropic.MessageParam[]>(messages);
  const anthropicResponse = await anthropic.messages.create({
    model: "claude-haiku-4-5-20251001",
    messages: anthropicMessages,
    max_tokens: 1000,
  });

  return [anthropicResponse];
};

// ============================================================================
// Wordle Solver Agent
// ============================================================================

type WordleFeedback = {
  guess: string;
  result: string; // e.g., "🟩🟨⬜⬜🟩"
};

type WordleGameState = {
  targetWord: string;
  guesses: WordleFeedback[];
  maxGuesses: number;
};

function evaluateGuess(guess: string, target: string): string {
  const result: string[] = [];
  const targetLetters = target.split("");
  const guessLetters = guess.toUpperCase().split("");

  // Track which target letters have been matched
  const matched = new Array(target.length).fill(false);

  // First pass: mark correct positions (green)
  for (let i = 0; i < guessLetters.length; i++) {
    if (guessLetters[i] === targetLetters[i]) {
      result[i] = "🟩";
      matched[i] = true;
    }
  }

  // Second pass: mark wrong positions (yellow) and misses (gray)
  for (let i = 0; i < guessLetters.length; i++) {
    if (result[i] === "🟩") continue;

    // Check if letter exists elsewhere
    const letterIndex = targetLetters.findIndex(
      (letter, idx) => letter === guessLetters[i] && !matched[idx]
    );

    if (letterIndex !== -1) {
      result[i] = "🟨";
      matched[letterIndex] = true;
    } else {
      result[i] = "⬜";
    }
  }

  return result.join("");
}

function displayWordleBoard(gameState: WordleGameState): string {
  let board = "\n";
  board += "  ┌───────────────────┐\n";

  for (const feedback of gameState.guesses) {
    board += `  │ ${feedback.guess.toUpperCase()}  ${feedback.result} │\n`;
  }

  // Show remaining empty rows
  const remaining = gameState.maxGuesses - gameState.guesses.length;
  for (let i = 0; i < remaining; i++) {
    board += "  │        . . . . .  │\n";
  }

  board += "  └───────────────────┘\n";
  return board;
}

async function wordleSolverAgent() {
  const gameState: WordleGameState = {
    targetWord: "PIZZA",
    guesses: [],
    maxGuesses: 10,
  };

  console.log("\n" + "═".repeat(COL_WIDTH));
  console.log(centerText("🎮 Wordle Solver Agent", COL_WIDTH));
  console.log("═".repeat(COL_WIDTH));
  console.log(
    "\nThe agent will strategically solve a Wordle puzzle using tool calls."
  );
  console.log("Watch as it reasons about each guess!\n");

  // Define the tool in Lingua's universal format
  const tools: Tool[] = [
    clientTool({
      name: "make_guess",
      description:
        "Solve the Wordle puzzle. Each guess *must* be a valid 5-letter word.",
      input_schema: {
        type: "object",
        properties: {
          word: {
            type: "string",
            description: "A 5-letter word guess",
          },
        },
        required: ["word"],
      },
    }),
  ];

  // Initialize conversation
  const messages: Message[] = [
    {
      role: "user",
      content: `Let's play Wordle! You have ${gameState.maxGuesses} guesses to find a 5-letter word.

Rules:
- 🟩 = correct letter in correct position
- 🟨 = correct letter in wrong position
- ⬜ = letter not in word

Use the make_guess tool to make your guesses. Think strategically!`,
    },
  ];

  // Randomly choose provider for this game
  const useOpenAI = Math.random() > 0.5;
  const providerName = useOpenAI ? "OpenAI" : "Anthropic";

  console.log(`🎲 Randomly selected provider: ${providerName}\n`);
  console.log("💡 The same tools and messages work with ANY provider!\n");

  let turnCount = 0;
  const maxTurns = 10; // Safety limit

  while (
    gameState.guesses.length < gameState.maxGuesses &&
    turnCount < maxTurns
  ) {
    turnCount++;

    console.log(`\n${"─".repeat(COL_WIDTH)}`);
    console.log(`Turn ${turnCount}:`);

    // Call the LLM with tools
    let response: Message[];

    if (useOpenAI) {
      const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
      const openaiMessages =
        linguaToChatCompletionsMessages<
          OpenAI.Chat.ChatCompletionMessageParam[]
        >(messages);
      const openaiTools = linguaToolsToOpenAI(tools);

      const completion = await openai.chat.completions.create({
        model: "gpt-5-mini",
        messages: openaiMessages,
        tools: openaiTools,
        tool_choice: "auto",
      });

      response = chatCompletionsMessagesToLingua([
        completion.choices[0].message,
      ]);
    } else {
      const anthropic = new Anthropic({
        apiKey: process.env.ANTHROPIC_API_KEY,
      });
      const anthropicMessages =
        linguaToAnthropicMessages<Anthropic.MessageParam[]>(messages);
      const anthropicTools = linguaToolsToAnthropic(tools);

      const completion = await anthropic.messages.create({
        model: "claude-haiku-4-5-20251001",
        messages: anthropicMessages,
        tools: anthropicTools,
        max_tokens: 1000,
      });

      response = anthropicMessagesToLingua([completion]);
    }

    messages.push(response[0]);

    // Check if the assistant wants to use a tool
    const assistantMessage = response[0];
    if (
      typeof assistantMessage.content !== "string" &&
      Array.isArray(assistantMessage.content)
    ) {
      const toolCalls = assistantMessage.content.filter(
        (block) => block.type === "tool_call"
      );

      if (toolCalls.length > 0) {
        // Process tool calls
        for (const toolCall of toolCalls) {
          if (toolCall.type !== "tool_call") continue;

          // Parse arguments - handle both string and ToolCallArguments types
          let args: any;
          if (typeof toolCall.arguments === "string") {
            args = JSON.parse(toolCall.arguments);
          } else if (
            typeof toolCall.arguments === "object" &&
            "type" in toolCall.arguments &&
            toolCall.arguments.type === "valid"
          ) {
            args = toolCall.arguments.value;
          } else {
            args = toolCall.arguments;
          }

          console.log(`\n🤔 Agent wants to guess: ${args.word.toUpperCase()}`);

          // Execute the tool
          const guess = args.word.toUpperCase();

          // Validate guess length
          if (guess.length !== 5) {
            console.log("❌ Invalid guess length!");

            // Return error feedback without consuming a guess
            messages.push({
              role: "tool",
              content: [
                {
                  type: "tool_result",
                  tool_call_id: toolCall.tool_call_id,
                  tool_name: toolCall.tool_name,
                  output: {
                    error: `Invalid guess: "${args.word}" is not exactly 5 letters. Please provide a valid 5-letter word.`,
                    guesses_remaining:
                      gameState.maxGuesses - gameState.guesses.length,
                  },
                },
              ],
            });
            continue;
          }

          const result = evaluateGuess(guess, gameState.targetWord);
          gameState.guesses.push({ guess, result });

          // Display board
          console.log(displayWordleBoard(gameState));

          // Add tool result to conversation
          messages.push({
            role: "tool",
            content: [
              {
                type: "tool_result",
                tool_call_id: toolCall.tool_call_id,
                tool_name: toolCall.tool_name,
                output: {
                  guess,
                  result,
                  board: displayWordleBoard(gameState),
                  guesses_remaining:
                    gameState.maxGuesses - gameState.guesses.length,
                },
              },
            ],
          });

          // Check if solved
          if (result === "🟩🟩🟩🟩🟩") {
            console.log(
              `\n🎉 Solved in ${gameState.guesses.length} guess${gameState.guesses.length === 1 ? "" : "es"}!`
            );
            console.log("═".repeat(COL_WIDTH));
            return;
          }
        }
      } else {
        // No tool calls, agent might be done or confused
        break;
      }
    } else {
      // Text-only response, no tool call
      console.log("\n💬 Agent response:", assistantMessage.content);
      break;
    }
  }

  if (
    gameState.guesses[gameState.guesses.length - 1]?.result !== "🟩🟩🟩🟩🟩"
  ) {
    console.log(
      `\n😅 Not solved in ${gameState.maxGuesses} guesses. The word was: ${gameState.targetWord}`
    );
  }

  console.log("═".repeat(COL_WIDTH));
}

const COL_WIDTH = 80;

main().catch(console.error);
