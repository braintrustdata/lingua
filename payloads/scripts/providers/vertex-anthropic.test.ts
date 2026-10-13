import { afterEach, describe, expect, test, vi } from "vitest";
import { readFileSync } from "fs";
import { loadServiceAccountKey } from "./vertex-anthropic";

vi.mock("fs", async (importOriginal) => ({
  ...(await importOriginal<typeof import("fs")>()),
  readFileSync: vi.fn(),
}));

const key = {
  client_email: "capture@example.invalid",
  private_key: "test-private-key",
  token_uri: "https://oauth2.googleapis.com/token",
};

afterEach(() => {
  vi.unstubAllEnvs();
  vi.resetAllMocks();
});

describe("Vertex service account credentials", () => {
  test("reads JSON from the secret variable without reading a file", () => {
    vi.stubEnv("GOOGLE_SERVICE_ACCOUNT_JSON", JSON.stringify(key));
    vi.stubEnv("GOOGLE_APPLICATION_CREDENTIALS", "/unused/credentials.json");
    expect(loadServiceAccountKey()).toEqual(key);
    expect(readFileSync).not.toHaveBeenCalled();
  });

  test("preserves the credential file path option", () => {
    vi.stubEnv("GOOGLE_SERVICE_ACCOUNT_JSON", undefined);
    vi.stubEnv("GOOGLE_APPLICATION_CREDENTIALS", "/credentials.json");
    vi.mocked(readFileSync).mockReturnValue(JSON.stringify(key));
    expect(loadServiceAccountKey()).toEqual(key);
    expect(readFileSync).toHaveBeenCalledWith("/credentials.json", "utf-8");
  });

  test("rejects invalid secret JSON without exposing it or trying the file", () => {
    vi.stubEnv("GOOGLE_SERVICE_ACCOUNT_JSON", "private-secret-invalid-json");
    vi.stubEnv("GOOGLE_APPLICATION_CREDENTIALS", "/credentials.json");
    expect(() => loadServiceAccountKey()).toThrow(
      "Invalid service account JSON in GOOGLE_SERVICE_ACCOUNT_JSON"
    );
    expect(readFileSync).not.toHaveBeenCalled();
  });

  test.each(["client_email", "private_key", "token_uri"])(
    "rejects credentials missing %s",
    (field) => {
      const incomplete: Record<string, string> = { ...key };
      delete incomplete[field];
      vi.stubEnv("GOOGLE_SERVICE_ACCOUNT_JSON", JSON.stringify(incomplete));
      expect(() => loadServiceAccountKey()).toThrow(field);
    }
  );

  test("names both supported options when credentials are missing", () => {
    vi.stubEnv("GOOGLE_SERVICE_ACCOUNT_JSON", undefined);
    vi.stubEnv("GOOGLE_APPLICATION_CREDENTIALS", undefined);
    expect(() => loadServiceAccountKey()).toThrow(
      "GOOGLE_SERVICE_ACCOUNT_JSON or GOOGLE_APPLICATION_CREDENTIALS"
    );
  });
});
