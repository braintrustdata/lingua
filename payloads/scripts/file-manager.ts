import { writeFileSync, mkdirSync } from "fs";
import { join } from "path";
import { CaptureResult } from "./types";

export function createTestCaseDirectory(baseOutputDir: string, testCase: string, provider: string): string {
  const testCaseDir = join(baseOutputDir, testCase, provider);
  mkdirSync(testCaseDir, { recursive: true });
  return testCaseDir;
}

export function saveAllFiles(
  outputDir: string,
  testCase: string,
  provider: string,
  result: CaptureResult
): string[] {
  const testCaseDir = createTestCaseDirectory(outputDir, testCase, provider);
  const savedFiles: string[] = [];

  // Always save the request
  const requestPath = join(testCaseDir, 'request.json');
  writeFileSync(requestPath, JSON.stringify(result.request, null, 2));
  savedFiles.push(requestPath);

  // Save response if it exists
  if (result.response) {
    const responsePath = join(testCaseDir, 'response.json');
    writeFileSync(responsePath, JSON.stringify(result.response, null, 2));
    savedFiles.push(responsePath);
  }

  // Save streaming response if it exists
  if (result.streamingResponse) {
    const streamingPath = join(testCaseDir, 'response-streaming.json');
    writeFileSync(streamingPath, JSON.stringify(result.streamingResponse, null, 2));
    savedFiles.push(streamingPath);
  }

  // Save follow-up request if it exists
  if (result.followupRequest) {
    const followupRequestPath = join(testCaseDir, 'followup-request.json');
    writeFileSync(followupRequestPath, JSON.stringify(result.followupRequest, null, 2));
    savedFiles.push(followupRequestPath);
  }

  // Save follow-up response if it exists
  if (result.followupResponse) {
    const followupResponsePath = join(testCaseDir, 'followup-response.json');
    writeFileSync(followupResponsePath, JSON.stringify(result.followupResponse, null, 2));
    savedFiles.push(followupResponsePath);
  }

  // Save follow-up streaming response if it exists
  if (result.followupStreamingResponse) {
    const followupStreamingPath = join(testCaseDir, 'followup-response-streaming.json');
    writeFileSync(followupStreamingPath, JSON.stringify(result.followupStreamingResponse, null, 2));
    savedFiles.push(followupStreamingPath);
  }

  // Save error if it exists
  if (result.error) {
    const errorPath = join(testCaseDir, 'error.json');
    writeFileSync(errorPath, JSON.stringify({ error: result.error }, null, 2));
    savedFiles.push(errorPath);
  }

  return savedFiles;
}