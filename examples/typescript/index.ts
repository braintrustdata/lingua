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
  console.log("\n" + "═".repeat(COL_WIDTH));
  console.log(centerText("📝 Simple Text Completion 📝", COL_WIDTH));
  console.log("═".repeat(COL_WIDTH));

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

const createOpenAiCompletion = async (messages: Message[], tools?: Tool[]) => {
  const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
  const openaiMessages =
    linguaToChatCompletionsMessages<OpenAI.Chat.ChatCompletionMessageParam[]>(
      messages
    );
  const openAiResponse = await openai.chat.completions.create({
    model: "gpt-5-nano",
    messages: openaiMessages,
    ...(tools && {
      tools: linguaToolsToOpenAI(tools),
      tool_choice: "required",
    }),
    reasoning_effort: "low",
  });

  return [openAiResponse.choices[0].message];
};

const createAnthropicCompletion = async (
  messages: Message[],
  tools?: Tool[]
) => {
  const anthropic = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY });
  const anthropicMessages =
    linguaToAnthropicMessages<Anthropic.MessageParam[]>(messages);
  const anthropicResponse = await anthropic.messages.create({
    model: "claude-haiku-4-5-20251001",
    messages: anthropicMessages,
    max_tokens: 1000,
    ...(tools && {
      tools: linguaToolsToAnthropic(tools),
      tool_choice: {
        type: "any",
      },
    }),
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

function displayWordleBoard(
  gameState: WordleGameState,
  maxGuesses: number
): string {
  let board = "\n";
  board += "  ┌───────────────────┐\n";

  for (const feedback of gameState.guesses) {
    board += `  │ ${feedback.guess.toUpperCase()}  ${feedback.result} │\n`;
  }

  // Show remaining empty rows
  const remaining = maxGuesses - gameState.guesses.length;
  for (let i = 0; i < remaining; i++) {
    board += "  │        . . . . .  │\n";
  }

  board += "  └───────────────────┘\n";
  return board;
}

async function wordleSolverAgent() {
  const gameState: WordleGameState = {
    targetWord: "SAUCE",
    guesses: [],
    maxGuesses: 10,
  };

  console.log("\n" + "═".repeat(COL_WIDTH));
  console.log(centerText("🧩 Wordle Solver Agent 🧩", COL_WIDTH));
  console.log("═".repeat(COL_WIDTH));
  console.log("\nTime to play Wordle 🤓");
  console.log(
    "\nWe'll use a multi-model agent loop that alternates between OpenAI and Anthropic each turn!\n"
  );

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

You'll receive feedback on each guess in the form of a string of emojis:
- 🟩 = correct letter in correct position
- 🟨 = correct letter in wrong position
- ⬜ = letter not in word

* Remember that if a letter is green, you should lock it in place for all remaining guesses.
* Just play the game, do not explain anything to me or provide any commentary.
* Use the make_guess tool to make your guesses.
* You must make a guess on each response!`,
    },
  ];

  let turnCount = 0;
  const maxTurns = 20;
  let guessesCount = 0;
  const maxGuesses = 10;

  while (turnCount < maxTurns && guessesCount < maxGuesses) {
    turnCount++;

    // Alternate providers: OpenAI on odd turns, Anthropic on even turns
    const providerName = turnCount % 2 === 1 ? "Anthropic" : "OpenAI";

    console.log(`Turn ${turnCount} (${providerName}):`);

    // We'll maintain our chat thread in Lingua format, allowing us to switch providers seamlessly
    const response =
      providerName === "OpenAI"
        ? chatCompletionsMessagesToLingua(
            await createOpenAiCompletion(messages, tools)
          )
        : anthropicMessagesToLingua(
            await createAnthropicCompletion(messages, tools)
          );

    const message = response[0]; // The helpers used above always return an array

    messages.push(message);

    // Check if the assistant wants to use a tool
    const assistantMessage = message;
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

          console.log(`\n🤔 Agent's guess: ${args.word.toUpperCase()}`);

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
                    guesses_remaining: maxGuesses - guessesCount,
                  },
                },
              ],
            });

            continue;
          }

          guessesCount++; // Increment only after a valid guess
          const result = evaluateGuess(guess, gameState.targetWord);
          gameState.guesses.push({ guess, result });

          // Display board
          console.log(displayWordleBoard(gameState, maxGuesses));

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
                  board: displayWordleBoard(gameState, maxGuesses),
                  guesses_remaining: maxGuesses - guessesCount,
                },
              },
            ],
          });

          // Check if solved
          if (result === "🟩🟩🟩🟩🟩") {
            console.log(
              `\n🎉 Solved in ${gameState.guesses.length} guess${gameState.guesses.length === 1 ? "" : "es"}!`
            );

            return;
          }
        }
      } else {
        // No valid tool calls found
        // Have the agent try again
        messages.push({
          role: "user",
          content:
            "Please make sure you call the make_guess tool each time you respond!",
        });
        console.log(
          "\n💬 No tool call found! Agent response:",
          assistantMessage.content
        );

        continue;
      }
    } else {
      // Text-only response, no tool call
      // Have the agent try again
      messages.push({
        role: "user",
        content:
          "Please make sure you call the make_guess tool each time you respond!",
      });
      console.log(
        "\n💬 No tool call found! Agent response:",
        assistantMessage.content
      );

      continue;
    }
  }

  if (
    gameState.guesses[gameState.guesses.length - 1]?.result !== "🟩🟩🟩🟩🟩"
  ) {
    console.log(
      `\n😅 Not solved in ${maxGuesses} guesses. The word was: ${gameState.targetWord}`
    );
  }

  console.log("═".repeat(COL_WIDTH));
}

const COL_WIDTH = 100;

main().catch(console.error);
