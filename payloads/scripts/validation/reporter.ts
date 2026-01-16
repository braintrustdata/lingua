// Console output formatting for validation results

import { ValidationResult } from "./index";
import { formatDiff } from "./diff-utils";

// ANSI color codes
const colors = {
  reset: "\x1b[0m",
  green: "\x1b[32m",
  red: "\x1b[31m",
  yellow: "\x1b[33m",
  cyan: "\x1b[36m",
  dim: "\x1b[2m",
  bold: "\x1b[1m",
};

interface PrinterOptions {
  verbose?: boolean;
  proxyUrl: string;
}

/**
 * Create a streaming printer that prints results as they complete.
 */
export function createStreamingPrinter(options: PrinterOptions) {
  const { verbose, proxyUrl } = options;
  let currentFormat: string | null = null;
  let headerPrinted = false;

  return {
    printResult(result: ValidationResult): void {
      // Print header on first result
      if (!headerPrinted) {
        console.log(
          `\nValidating proxy at ${colors.cyan}${proxyUrl}${colors.reset}...\n`
        );
        headerPrinted = true;
      }

      // Print format header when format changes
      if (result.format !== currentFormat) {
        if (currentFormat !== null) {
          console.log(); // Blank line between formats
        }
        console.log(`${colors.bold}${result.format}${colors.reset}`);
        currentFormat = result.format;
      }

      // Print result
      const icon = result.success
        ? `${colors.green}✓${colors.reset}`
        : `${colors.red}✗${colors.reset}`;
      const duration = `${colors.dim}(${result.durationMs}ms)${colors.reset}`;
      const modelLabel =
        result.model !== "default"
          ? ` ${colors.cyan}[${result.model}]${colors.reset}`
          : "";

      if (result.success) {
        console.log(`  ${icon} ${result.caseName}${modelLabel} ${duration}`);
      } else if (result.error) {
        console.log(`  ${icon} ${result.caseName}${modelLabel} ${duration}`);
        console.log(`    ${colors.red}Error: ${result.error}${colors.reset}`);
      } else if (result.diff) {
        console.log(`  ${icon} ${result.caseName}${modelLabel} ${duration}`);
        console.log(`    ${colors.yellow}Differences found:${colors.reset}`);

        const diffsToShow = verbose
          ? result.diff.diffs
          : result.diff.diffs.slice(0, 5);
        for (const diff of diffsToShow) {
          console.log(`      ${colors.dim}${formatDiff(diff)}${colors.reset}`);
        }

        if (!verbose && result.diff.diffs.length > 5) {
          console.log(
            `      ${colors.dim}... and ${result.diff.diffs.length - 5} more differences${colors.reset}`
          );
        }
      }
    },

    printSummary(results: ValidationResult[]): void {
      const passed = results.filter((r) => r.success).length;
      const failed = results.length - passed;
      const totalDuration = results.reduce((sum, r) => sum + r.durationMs, 0);

      console.log(); // Blank line before summary
      console.log("━".repeat(50));
      console.log(
        `${colors.bold}Summary:${colors.reset} ${colors.green}${passed} passed${colors.reset}, ${failed > 0 ? colors.red : colors.dim}${failed} failed${colors.reset}`
      );
      console.log(`${colors.dim}Total time: ${totalDuration}ms${colors.reset}`);
      console.log("━".repeat(50));
    },
  };
}

/**
 * Print a validation report to the console (batch mode).
 * @deprecated Use createStreamingPrinter for streaming output
 */
export function printReport(
  results: ValidationResult[],
  options: PrinterOptions
): void {
  const printer = createStreamingPrinter(options);
  for (const result of results) {
    printer.printResult(result);
  }
  printer.printSummary(results);
}
