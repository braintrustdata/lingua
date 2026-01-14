#!/usr/bin/env tsx

// CLI wrapper for the validation library

import {
  runValidation,
  getAvailableFormats,
  ValidationResult,
} from "./validation";
import { createStreamingPrinter } from "./validation/reporter";

interface CLIOptions {
  proxyUrl?: string;
  apiKey?: string;
  formats?: string[];
  cases?: string[];
  providers?: string[];
  all: boolean;
  verbose: boolean;
  stream: boolean;
  help: boolean;
}

function parseArguments(): CLIOptions {
  const args = process.argv.slice(2);
  const options: CLIOptions = {
    all: false,
    verbose: false,
    stream: false,
    help: false,
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    switch (arg) {
      case "--proxy-url":
        if (i + 1 < args.length) {
          options.proxyUrl = args[i + 1];
          i++;
        }
        break;
      case "--api-key":
        if (i + 1 < args.length) {
          options.apiKey = args[i + 1];
          i++;
        }
        break;
      case "--format":
        if (i + 1 < args.length) {
          options.formats = args[i + 1].split(",");
          i++;
        }
        break;
      case "--cases":
        if (i + 1 < args.length) {
          options.cases = args[i + 1].split(",");
          i++;
        }
        break;
      case "--providers":
        if (i + 1 < args.length) {
          options.providers = args[i + 1].split(",");
          i++;
        }
        break;
      case "--all":
      case "-a":
        options.all = true;
        break;
      case "--verbose":
      case "-v":
        options.verbose = true;
        break;
      case "--stream":
      case "-s":
        options.stream = true;
        break;
      case "--help":
      case "-h":
        options.help = true;
        break;
      default:
        if (arg.startsWith("--")) {
          console.error(`Unknown option: ${arg}`);
          process.exit(1);
        }
    }
  }

  return options;
}

function printHelp(): void {
  const formats = getAvailableFormats();

  console.log(`
Usage: ./scripts/validate.ts --proxy-url <url> [options]

Validate an LLM proxy by comparing responses to captured snapshots.

Required:
  --proxy-url <url>      Proxy URL (e.g., http://localhost:8080)

Options:
  --api-key <key>        API key to use (default: provider-specific env vars)
                         Use $BRAINTRUST_API_KEY for Braintrust gateway
  --format <formats>     Comma-separated formats to test (default: chat-completions)
                         Available: ${formats.join(", ")}
  --providers <providers> Comma-separated providers to test (default: uses snapshot model)
                         Available: openai, anthropic
  --cases <cases>        Comma-separated case names or collections to test
                         Collections: simple, advanced, params
  --verbose, -v          Show full diff details
  --stream, -s           Test streaming responses instead of non-streaming
  --help, -h             Show this help message

Examples:
  # Validate chat-completions format through local gateway
  ./scripts/validate.ts --proxy-url http://localhost:8080

  # Validate with Braintrust API key
  ./scripts/validate.ts --proxy-url http://localhost:8080 --api-key $BRAINTRUST_API_KEY

  # Validate chat-completions format with both OpenAI and Anthropic providers
  ./scripts/validate.ts --proxy-url http://localhost:8080 --providers openai,anthropic

  # Validate multiple formats
  ./scripts/validate.ts --proxy-url http://localhost:8080 --format chat-completions,anthropic

  # Validate specific cases
  ./scripts/validate.ts --proxy-url http://localhost:8080 --cases simpleRequest,toolCallRequest

Environment Variables:
  BRAINTRUST_API_KEY     API key for Braintrust gateway
  OPENAI_API_KEY         API key for OpenAI (if not using --api-key)
  ANTHROPIC_API_KEY      API key for Anthropic (if not using --api-key)
`);
}

async function main(): Promise<void> {
  const options = parseArguments();

  if (options.help) {
    printHelp();
    process.exit(0);
  }

  if (!options.proxyUrl) {
    console.error("Error: --proxy-url is required\n");
    printHelp();
    process.exit(1);
  }

  // Use BRAINTRUST_API_KEY if available and no --api-key specified
  const apiKey = options.apiKey ?? process.env.BRAINTRUST_API_KEY;

  // Create streaming printer for real-time output
  const printer = createStreamingPrinter({
    verbose: options.verbose,
    proxyUrl: options.proxyUrl,
  });

  // Collect results while printing them as they complete
  const results: ValidationResult[] = [];

  await runValidation({
    proxyUrl: options.proxyUrl,
    apiKey,
    formats: options.formats,
    cases: options.cases,
    providers: options.providers,
    all: options.all,
    stream: options.stream,
    onResult: (result) => {
      results.push(result);
      printer.printResult(result);
    },
  });

  printer.printSummary(results);

  // Exit with error code only for actual failures (not warnings)
  // Warnings (minor diffs like logprobs, tool args) are acceptable variations
  const failed = results.filter((r) => !r.success).length;
  process.exit(failed > 0 ? 1 : 0);
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
