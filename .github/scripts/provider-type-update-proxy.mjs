import { randomBytes, timingSafeEqual } from "node:crypto";
import { createServer } from "node:http";
import { Readable, Transform } from "node:stream";
import { pipeline } from "node:stream/promises";

const upstream = "https://gateway.staging.braintrust.dev";
const paths = new Set(["/v1/messages", "/v1/messages/count_tokens"]);

function credentialFilter(key) {
  const needles = [
    ...new Set([
      key,
      encodeURIComponent(key),
      Buffer.from(key).toString("base64"),
    ]),
  ].map((value) => Buffer.from(value));
  const longest = Math.max(...needles.map((value) => value.length));
  let pending = Buffer.alloc(0);
  const emit = (stream, final) => {
    while (pending.length) {
      const safe = final
        ? pending.length
        : Math.max(0, pending.length - longest + 1);
      if (!safe) break;
      const matches = needles
        .map((needle) => ({ needle, index: pending.indexOf(needle) }))
        .filter(({ index }) => index >= 0 && index < safe)
        .sort((a, b) => a.index - b.index);
      if (matches.length) {
        const { needle, index } = matches[0];
        stream.push(pending.subarray(0, index));
        stream.push("[REDACTED]");
        pending = pending.subarray(index + needle.length);
      } else {
        stream.push(pending.subarray(0, safe));
        pending = pending.subarray(safe);
      }
    }
  };
  return new Transform({
    transform(chunk, encoding, done) {
      pending = Buffer.concat([pending, chunk]);
      emit(this, false);
      done();
    },
    flush(done) {
      emit(this, true);
      done();
    },
  });
}

// The container receives a short-lived capability for inference only. The
// reusable Braintrust credential stays in this host process, outside its PID,
// filesystem, and environment namespaces. There is no general HTTP forwarding.
export async function createInferenceProxy({
  key,
  fetchImpl = fetch,
  host = "0.0.0.0",
}) {
  if (!key) throw new Error("The inference proxy requires a credential.");
  const token = randomBytes(32).toString("hex");
  let remaining = 1500;
  let active = 0;
  const server = createServer(async (request, response) => {
    const reject = (status) => {
      response.writeHead(status, { "content-type": "application/json" });
      response.end(
        JSON.stringify({
          error: {
            type: "proxy_error",
            message: "Inference proxy rejected the request.",
          },
        })
      );
    };
    const supplied = Buffer.from(String(request.headers["x-api-key"] ?? ""));
    const expected = Buffer.from(token);
    if (
      supplied.length !== expected.length ||
      !timingSafeEqual(supplied, expected)
    ) {
      reject(401);
      return;
    }
    let url;
    try {
      url = new URL(request.url, "http://proxy.invalid");
    } catch {
      reject(403);
      return;
    }
    if (
      request.method !== "POST" ||
      !paths.has(url.pathname) ||
      !["", "?beta=true"].includes(url.search) ||
      request.url !== url.pathname + url.search
    ) {
      reject(403);
      return;
    }
    if (active >= 8 || --remaining < 0) {
      reject(429);
      return;
    }
    active++;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 600000);
    response.on("close", () => controller.abort());
    try {
      const chunks = [];
      let length = 0;
      for await (const chunk of request) {
        length += chunk.length;
        if (length > 16 * 1024 * 1024) {
          reject(413);
          return;
        }
        chunks.push(chunk);
      }
      const headers = {
        "content-type": "application/json",
        "anthropic-version": "2023-06-01",
        authorization: `Bearer ${key}`,
        "x-bt-project-name": "automations-spend-control",
      };
      const beta = request.headers["anthropic-beta"];
      if (typeof beta === "string" && /^[A-Za-z0-9,_.-]{1,2048}$/.test(beta)) {
        headers["anthropic-beta"] = beta;
      }
      const result = await fetchImpl(upstream + url.pathname + url.search, {
        method: "POST",
        headers,
        body: Buffer.concat(chunks),
        redirect: "manual",
        signal: controller.signal,
      });
      // Never expose upstream error bodies or headers, which can echo auth.
      if (!result.ok) {
        await result.body?.cancel();
        reject(
          result.status >= 400 && result.status <= 599 ? result.status : 502
        );
        return;
      }
      response.writeHead(200, {
        "content-type":
          result.headers.get("content-type") || "application/json",
      });
      if (result.body)
        await pipeline(
          Readable.fromWeb(result.body),
          credentialFilter(key),
          response
        );
      else response.end();
    } catch {
      if (!response.headersSent) reject(502);
      else response.destroy();
    } finally {
      active--;
      clearTimeout(timer);
    }
  });
  server.requestTimeout = 600000;
  server.maxConnections = 32;
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, host, resolve);
  });
  return {
    token,
    port: server.address().port,
    async close() {
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    },
  };
}
