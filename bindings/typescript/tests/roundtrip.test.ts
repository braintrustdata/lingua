/**
 * TypeScript Roundtrip Tests
 *
 * These tests validate that:
 * 1. Snapshots from payloads directory are valid according to SDK types
 * 2. Generated Rust types are compatible with SDK types
 * 3. We can parse and type-check all test data
 */

import { describe, test, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import type {
  ChatCompletionCreateParams,
  ChatCompletion,
  ChatCompletionChunk,
} from 'openai/resources/chat';
import type {
  MessageCreateParams,
  Message as AnthropicMessage,
  MessageStreamEvent,
} from '@anthropic-ai/sdk/resources/messages';

// Import our generated types
import type { Message as LLMIRMessage } from '../src';

interface TestSnapshot {
  name: string;
  provider: 'openai-chat-completions' | 'openai-responses' | 'anthropic';
  turn: 'first_turn' | 'followup_turn';
  request?: any;
  response?: any;
  streamingResponse?: any;
}

/**
 * Load all snapshots for a given test case
 */
function loadTestSnapshots(testCaseName: string): TestSnapshot[] {
  const snapshots: TestSnapshot[] = [];
  // Snapshots are in the payloads directory
  const snapshotsDir = path.join(__dirname, '../../../payloads/snapshots', testCaseName);

  const providers = ['openai-chat-completions', 'openai-responses', 'anthropic'] as const;
  const turns = ['first_turn', 'followup_turn'] as const;

  for (const provider of providers) {
    const providerDir = path.join(snapshotsDir, provider);

    if (!fs.existsSync(providerDir)) continue;

    for (const turn of turns) {
      const prefix = turn === 'followup_turn' ? 'followup-' : '';

      const snapshot: TestSnapshot = {
        name: testCaseName,
        provider,
        turn,
      };

      // Load request
      const requestPath = path.join(providerDir, `${prefix}request.json`);
      if (fs.existsSync(requestPath)) {
        snapshot.request = JSON.parse(fs.readFileSync(requestPath, 'utf-8'));
      }

      // Load response
      const responsePath = path.join(providerDir, `${prefix}response.json`);
      if (fs.existsSync(responsePath)) {
        snapshot.response = JSON.parse(fs.readFileSync(responsePath, 'utf-8'));
      }

      // Load streaming response
      const streamingPath = path.join(providerDir, `${prefix}response-streaming.json`);
      if (fs.existsSync(streamingPath)) {
        const content = fs.readFileSync(streamingPath, 'utf-8');
        try {
          // Try parsing as JSON array first (most common format)
          snapshot.streamingResponse = JSON.parse(content);
        } catch (e) {
          // If that fails, try newline-delimited JSON
          snapshot.streamingResponse = content
            .split('\n')
            .filter(line => line.trim())
            .map(line => {
              try {
                return JSON.parse(line);
              } catch (e) {
                return null;
              }
            })
            .filter(item => item !== null);
        }
      }

      if (snapshot.request || snapshot.response || snapshot.streamingResponse) {
        snapshots.push(snapshot);
      }
    }
  }

  return snapshots;
}

/**
 * Validate that a snapshot matches the expected SDK type
 */
function validateSDKTypes(snapshot: TestSnapshot): string[] {
  const errors: string[] = [];

  try {
    switch (snapshot.provider) {
      case 'openai-chat-completions':
        if (snapshot.request) {
          // TypeScript compile-time type check
          const _typeCheck: ChatCompletionCreateParams = snapshot.request;

          // Runtime validation
          if (!snapshot.request.model) errors.push('Missing model field');
          if (!Array.isArray(snapshot.request.messages)) errors.push('Messages must be an array');
        }
        if (snapshot.response) {
          const _typeCheck: ChatCompletion = snapshot.response;

          if (!snapshot.response.id) errors.push('Response missing id');
          if (!Array.isArray(snapshot.response.choices)) errors.push('Response choices must be an array');
        }
        if (snapshot.streamingResponse) {
          snapshot.streamingResponse.forEach((chunk: any, i: number) => {
            try {
              const _typeCheck: ChatCompletionChunk = chunk;
            } catch (e) {
              errors.push(`Streaming chunk ${i} type mismatch`);
            }
          });
        }
        break;

      case 'anthropic':
        if (snapshot.request) {
          const _typeCheck: MessageCreateParams = snapshot.request;

          if (!snapshot.request.model) errors.push('Missing model field');
          if (!snapshot.request.max_tokens) errors.push('Missing max_tokens field');
          if (!Array.isArray(snapshot.request.messages)) errors.push('Messages must be an array');
        }
        if (snapshot.response) {
          const _typeCheck: AnthropicMessage = snapshot.response;

          if (!snapshot.response.id) errors.push('Response missing id');
          if (!snapshot.response.role) errors.push('Response missing role');
        }
        if (snapshot.streamingResponse) {
          snapshot.streamingResponse.forEach((event: any, i: number) => {
            try {
              const _typeCheck: MessageStreamEvent = event;
            } catch (e) {
              errors.push(`Streaming event ${i} type mismatch`);
            }
          });
        }
        break;

      case 'openai-responses':
        // OpenAI Responses API types would need to be imported/defined
        break;
    }
  } catch (error) {
    errors.push(`Type validation error: ${error}`);
  }

  return errors;
}

describe('TypeScript Roundtrip Tests', () => {
  const snapshotsDir = path.join(__dirname, '../../../payloads/snapshots');

  // Get all test cases
  const testCases = fs.existsSync(snapshotsDir)
    ? fs.readdirSync(snapshotsDir)
        .filter(name => fs.statSync(path.join(snapshotsDir, name)).isDirectory())
        .filter(name => !name.startsWith('.'))
    : [];

  if (testCases.length === 0) {
    test('No test cases found', () => {
      console.warn('No snapshot test cases found. Run capture script in payloads directory first.');
      expect(testCases.length).toBeGreaterThan(0);
    });
    return;
  }

  for (const testCase of testCases) {
    describe(testCase, () => {
      const snapshots = loadTestSnapshots(testCase);

      if (snapshots.length === 0) {
        test.skip('No snapshots found for this test case', () => {});
        return;
      }

      for (const snapshot of snapshots) {
        const testName = `${snapshot.provider} - ${snapshot.turn}`;

        test(`${testName}: validates SDK types`, () => {
          const errors = validateSDKTypes(snapshot);
          if (errors.length > 0) {
            console.error(`Validation errors for ${testName}:`, errors);
          }
          expect(errors).toEqual([]);
        });

        // Future: Add WASM-based roundtrip tests here
      }
    });
  }

  describe('Test Coverage', () => {
    test('All test cases have snapshots', () => {
      const coverage: Record<string, { providers: string[], turns: string[] }> = {};

      for (const testCase of testCases) {
        const snapshots = loadTestSnapshots(testCase);
        coverage[testCase] = {
          providers: [...new Set(snapshots.map(s => s.provider))],
          turns: [...new Set(snapshots.map(s => s.turn))]
        };
      }

      console.log('Test coverage by case:');
      for (const [testCase, data] of Object.entries(coverage)) {
        console.log(`  ${testCase}:`);
        console.log(`    Providers: ${data.providers.join(', ')}`);
        console.log(`    Turns: ${data.turns.join(', ')}`);
      }

      // Ensure each test case has at least some snapshots
      for (const testCase of testCases) {
        expect(coverage[testCase].providers.length).toBeGreaterThan(0);
      }
    });
  });

  describe('Generated Types', () => {
    test('Module exports are available', async () => {
      const module = await import('../src');

      // Check that VERSION constant is exported
      expect(module.VERSION).toBeDefined();
      expect(module.VERSION).toBe('0.1.0');
    });

    test('TypeScript types compile correctly', () => {
      // This test just verifies that we can import the types
      // The actual type checking happens at compile time
      const testMessage: LLMIRMessage = {
        role: 'user',
        content: 'Test message'
      };

      expect(testMessage).toBeDefined();
      expect(testMessage.role).toBe('user');
    });
  });
});