import { createHash } from "crypto";
import { existsSync, readFileSync, writeFileSync } from "fs";
import { join } from "path";

interface SnapshotMetadata {
  payloadHash: string;
  capturedAt: string;
  provider: string;
  name: string;
  files: string[];
}

interface SnapshotCache {
  [caseKey: string]: SnapshotMetadata;
}

const CACHE_FILE_NAME = ".snapshot-cache.json";

export function getPayloadHash(payload: unknown): string {
  const payloadStr = JSON.stringify(payload, null, 0);
  return createHash("sha256").update(payloadStr).digest("hex");
}

export function getCacheFilePath(outputDir: string): string {
  return join(outputDir, CACHE_FILE_NAME);
}

export function loadCache(outputDir: string): SnapshotCache {
  const cacheFilePath = getCacheFilePath(outputDir);
  if (!existsSync(cacheFilePath)) {
    return {};
  }

  try {
    const cacheContent = readFileSync(cacheFilePath, "utf-8");
    return JSON.parse(cacheContent) as SnapshotCache;
  } catch (error) {
    console.warn(`Warning: Failed to load cache file: ${error}`);
    return {};
  }
}

export function saveCache(outputDir: string, cache: SnapshotCache): void {
  const cacheFilePath = getCacheFilePath(outputDir);
  try {
    writeFileSync(cacheFilePath, JSON.stringify(cache, null, 2));
  } catch (error) {
    console.warn(`Warning: Failed to save cache file: ${error}`);
  }
}

export function getCacheKey(provider: string, name: string): string {
  return `${provider}:${name}`;
}

export function needsRegeneration(
  outputDir: string,
  provider: string,
  name: string,
  payload: unknown
): boolean {
  const cache = loadCache(outputDir);
  const cacheKey = getCacheKey(provider, name);
  const currentHash = getPayloadHash(payload);

  const metadata = cache[cacheKey];
  if (!metadata) {
    return true; // No cache entry, needs generation
  }

  if (metadata.payloadHash !== currentHash) {
    return true; // Payload changed, needs regeneration
  }

  // Check if all expected files exist
  const missingFiles = metadata.files.filter(
    (file) => !existsSync(join(outputDir, file))
  );

  if (missingFiles.length > 0) {
    console.log(
      `Cache hit but missing files for ${provider}/${name}: ${missingFiles.join(", ")}`
    );
    return true; // Files missing, needs regeneration
  }

  return false; // Cache hit and all files exist
}

export function updateCache(
  outputDir: string,
  provider: string,
  name: string,
  payload: unknown,
  generatedFiles: string[]
): void {
  const cache = loadCache(outputDir);
  const cacheKey = getCacheKey(provider, name);

  cache[cacheKey] = {
    payloadHash: getPayloadHash(payload),
    capturedAt: new Date().toISOString(),
    provider,
    name,
    files: generatedFiles,
  };

  saveCache(outputDir, cache);
}
