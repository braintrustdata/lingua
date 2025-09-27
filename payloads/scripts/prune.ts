#!/usr/bin/env tsx

/**
 * Prune snapshots that don't correspond to test cases defined in the code.
 *
 * This script removes snapshot files that no longer have corresponding test cases
 * in the build.rs file, helping keep the snapshots directory clean.
 */

import { readdir, stat, unlink, rmdir } from 'fs/promises';
import { join } from 'path';

const SNAPSHOTS_DIR = join(__dirname, '..', 'snapshots');
const PROJECT_ROOT = join(__dirname, '..', '..');

// Provider directories that are recognized by the build script
const PROVIDERS = ['openai-responses', 'openai-chat-completions', 'anthropic'] as const;

type Provider = typeof PROVIDERS[number];
type Turn = 'first_turn' | 'followup_turn';

interface ValidTestCase {
  caseName: string;
  provider: Provider;
  turn: Turn;
}

/**
 * Discover valid test cases by simulating the build.rs logic
 */
async function discoverValidTestCases(): Promise<ValidTestCase[]> {
  const validCases: ValidTestCase[] = [];

  try {
    const entries = await readdir(SNAPSHOTS_DIR);

    for (const entry of entries) {
      const casePath = join(SNAPSHOTS_DIR, entry);
      const entryStat = await stat(casePath);

      if (!entryStat.isDirectory()) continue;

      // Skip hidden directories and cache files
      if (entry.startsWith('.')) continue;

      const caseName = entry;

      // Check each provider directory
      for (const provider of PROVIDERS) {
        const providerDir = join(casePath, provider);

        try {
          const providerStat = await stat(providerDir);
          if (!providerStat.isDirectory()) continue;

          // Check for first turn (required: request.json)
          try {
            await stat(join(providerDir, 'request.json'));
            validCases.push({ caseName, provider, turn: 'first_turn' });
          } catch {
            // request.json doesn't exist, skip first turn
          }

          // Check for followup turn (required: followup-request.json)
          try {
            await stat(join(providerDir, 'followup-request.json'));
            validCases.push({ caseName, provider, turn: 'followup_turn' });
          } catch {
            // followup-request.json doesn't exist, skip followup turn
          }

        } catch {
          // Provider directory doesn't exist, skip
        }
      }
    }
  } catch (error) {
    console.error(`Error scanning snapshots directory: ${error}`);
    process.exit(1);
  }

  return validCases;
}

/**
 * Check if a snapshot file corresponds to a valid test case
 */
function isValidSnapshot(
  caseName: string,
  provider: Provider,
  filename: string,
  validCases: ValidTestCase[]
): boolean {
  // Determine which turn this file belongs to
  const turn: Turn = filename.startsWith('followup-') ? 'followup_turn' : 'first_turn';

  // Check if this combination exists in valid cases
  return validCases.some(
    testCase => testCase.caseName === caseName &&
                testCase.provider === provider &&
                testCase.turn === turn
  );
}

/**
 * Check if directory is empty
 */
async function isEmptyDirectory(dirPath: string): Promise<boolean> {
  try {
    const entries = await readdir(dirPath);
    return entries.length === 0;
  } catch {
    return true; // If we can't read it, consider it empty/non-existent
  }
}

/**
 * Main prune function
 */
async function pruneOrphanedSnapshots(): Promise<void> {
  console.log('🗑️  Pruning snapshots that don\'t correspond to test cases...');
  console.log(`🔍 Scanning snapshots directory: ${SNAPSHOTS_DIR}`);

  const validCases = await discoverValidTestCases();
  console.log(`✅ Found ${validCases.length} valid test cases`);

  // Debug: show what test cases were found
  if (process.env.DEBUG) {
    console.log('Valid test cases:');
    validCases.forEach(tc =>
      console.log(`  - ${tc.caseName}/${tc.provider}/${tc.turn}`)
    );
  }

  let totalSnapshots = 0;
  let deletedCount = 0;

  try {
    const caseEntries = await readdir(SNAPSHOTS_DIR);

    for (const caseEntry of caseEntries) {
      const casePath = join(SNAPSHOTS_DIR, caseEntry);
      const caseStat = await stat(casePath);

      if (!caseStat.isDirectory()) continue;

      // Skip hidden directories and cache files
      if (caseEntry.startsWith('.')) continue;

      const caseName = caseEntry;

      // Check each provider directory
      for (const provider of PROVIDERS) {
        const providerDir = join(casePath, provider);

        try {
          const providerStat = await stat(providerDir);
          if (!providerStat.isDirectory()) continue;

          // Check each file in the provider directory
          const files = await readdir(providerDir);

          for (const file of files) {
            const filePath = join(providerDir, file);
            const fileStat = await stat(filePath);

            if (!fileStat.isFile()) continue;

            totalSnapshots++;

            const isValid = isValidSnapshot(caseName, provider, file, validCases);

            if (process.env.DEBUG) {
              console.log(`  File: ${caseName}/${provider}/${file} - Valid: ${isValid}`);
            }

            if (!isValid) {
              console.log(`🗑️  Deleting orphaned snapshot: ${filePath}`);
              await unlink(filePath);
              deletedCount++;
            }
          }

          // Remove empty provider directories
          if (await isEmptyDirectory(providerDir)) {
            console.log(`🗑️  Removing empty provider directory: ${providerDir}`);
            await rmdir(providerDir);
          }

        } catch {
          // Provider directory doesn't exist or can't be accessed, skip
        }
      }

      // Remove empty case directories
      if (await isEmptyDirectory(casePath)) {
        console.log(`🗑️  Removing empty case directory: ${casePath}`);
        await rmdir(casePath);
      }
    }
  } catch (error) {
    console.error(`Error during pruning: ${error}`);
    process.exit(1);
  }

  console.log('✅ Pruning completed');
  console.log(`📊 Total snapshots scanned: ${totalSnapshots}`);
  console.log(`🗑️  Orphaned snapshots deleted: ${deletedCount}`);

  if (deletedCount === 0) {
    console.log('🎉 No orphaned snapshots found - all snapshots correspond to valid test cases!');
  }
}

// Run the script
if (require.main === module) {
  pruneOrphanedSnapshots().catch(error => {
    console.error(`Fatal error: ${error}`);
    process.exit(1);
  });
}