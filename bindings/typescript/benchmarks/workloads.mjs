import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

export function runDisplay(wasm, spans) {
  return spans.map((span) => {
    const convert = (messages) =>
      messages.flatMap((message) =>
        message.role === "additional_tools"
          ? []
          : wasm.lingua_to_chat_completions_messages([message])
      );
    const input = wasm.import_messages_from_spans([
      { input: span.input, metadata: span.metadata },
    ]);
    const output = wasm.import_messages_from_spans([{ output: span.output }]);
    return { input: convert(input), output: convert(output) };
  });
}

function readFixture(name) {
  return JSON.parse(
    readFileSync(
      new URL(
        `../../../crates/lingua/src/processing/trajectory/fixtures/${name}.json`,
        import.meta.url
      ),
      "utf8"
    )
  );
}

function scaledToolCycle(rounds) {
  const fixture = readFixture("chat-tool-cycle");
  const source = new Map(fixture.spans.map((span) => [span.id, span]));
  const spans = [structuredClone(source.get("root"))];
  const history = [];
  const turns = [];
  for (let round = 0; round < rounds; round++) {
    const call = structuredClone(source.get("call"));
    const tool = structuredClone(source.get("tool"));
    const final = structuredClone(source.get("final"));
    for (const span of [call, tool, final]) {
      span.id = `${span.id}-${round}`;
      span.span_id = span.id;
      span.metrics.start += round * 3;
      span.metrics.end += round * 3;
    }
    call.output.tool_calls[0].id = call.id;
    final.input[2].tool_calls[0].id = call.id;
    final.input[3].tool_call_id = call.id;
    tool.metadata = { tool_call_id: call.id };
    call.input = [call.input[0], ...history, ...call.input.slice(1)];
    const current = final.input.slice(1);
    final.input = [final.input[0], ...history, ...final.input.slice(1)];
    history.push(...current, ...final.output);
    spans.push(call, tool, final);
    turns.push({
      ...fixture.turns[0],
      request_id: call.id,
      response_id: final.id,
      work_ids: [call.id, tool.id],
    });
  }
  spans[0].metrics.end = rounds * 3 + 1;
  return { spans, turns };
}

export function trajectoryCases() {
  const fixtures = [
    "chat-tool-cycle",
    "responses-tool-cycle",
    "anthropic-tool-results-are-not-user-turns",
    "analysis-overlapping-conversation",
    "task-conversation-with-worker",
    "compaction-and-resumed-parent",
    "opaque-history-tool-continuation",
  ].map((name) => ({ name, fixture: readFixture(name) }));
  for (const rounds of [8, 32]) {
    fixtures.push({
      name: `Scaled tool cycle (${rounds} rounds, growing history)`,
      fixture: scaledToolCycle(rounds),
    });
  }
  return fixtures.flatMap(({ name, fixture }) =>
    [1, 16].map((batchSize) => ({
      suite: "trajectory",
      name: `${name}, batch ${batchSize}`,
      spans: fixture.spans,
      fixture,
      batchSize,
      byId: new Map(fixture.spans.map((span) => [span.id, span])),
    }))
  );
}

function scopeKey(scope) {
  return JSON.stringify([scope.root_span_id, scope.owner_span_id]);
}

function compareInstants(left, right) {
  const subMilliseconds = (value) =>
    Number(
      (/\.(\d+)/.exec(value)?.[1] ?? "").padEnd(9, "0").slice(3, 9) || "0"
    );
  return (
    Date.parse(left) - Date.parse(right) ||
    subMilliseconds(left) - subMilliseconds(right)
  );
}

class BrowserCollector {
  scopes = new Map();
  complete = false;

  scope(scope) {
    const collected = this.scopes.get(scopeKey(scope));
    if (!collected) throw new Error("Unknown trajectory scope");
    return collected;
  }

  turn(event) {
    const collected = this.scope(event.scope).turns.get(event.id);
    if (!collected) throw new Error("Unknown trajectory turn");
    return collected;
  }

  push(event) {
    if (this.complete) throw new Error("Trajectory collection has finished");
    switch (event.type) {
      case "start": {
        const key = scopeKey(event.scope);
        if (this.scopes.has(key)) throw new Error("Duplicate trajectory scope");
        this.scopes.set(key, {
          scope: event.scope,
          header: event.trajectory,
          turns: new Map(),
        });
        return;
      }
      case "turn": {
        const { turns } = this.scope(event.scope);
        if (turns.has(event.id)) throw new Error("Duplicate trajectory turn");
        turns.set(event.id, {
          turn: event.turn,
          work: new Map(),
          order: turns.size,
        });
        return;
      }
      case "request": {
        const collected = this.turn(event);
        collected.turn = {
          ...collected.turn,
          request_id: event.request_id,
          request: event.request,
          opaque_request: event.opaque_request?.length
            ? event.opaque_request
            : undefined,
        };
        return;
      }
      case "work":
        this.turn(event).work.set(event.position, event.step);
        return;
      case "response": {
        const collected = this.turn(event);
        collected.turn = {
          ...collected.turn,
          response_id: event.response_id ?? undefined,
          response: event.response ?? undefined,
          end_time: event.end_time ?? undefined,
          model: event.model ?? undefined,
        };
        return;
      }
      case "end_time": {
        const collected = this.turn(event);
        collected.turn = {
          ...collected.turn,
          end_time: event.end_time ?? undefined,
        };
        return;
      }
      case "interrupted": {
        const collected = this.turn(event);
        collected.turn = { ...collected.turn, interrupted: true };
        return;
      }
      case "failure":
      case "warning": {
        const scope = this.scope({
          root_span_id: event.root_span_id,
          owner_span_id: null,
        });
        const field =
          event.type === "failure" ? "import_failures" : "import_warnings";
        const diagnostics = scope.header.metadata[field] ?? [];
        if (!Array.isArray(diagnostics))
          throw new Error("Invalid trajectory diagnostics");
        scope.header = {
          ...scope.header,
          metadata: {
            ...scope.header.metadata,
            [field]: [
              ...diagnostics,
              { span_id: event.span_id, message: event.message },
            ],
          },
        };
        return;
      }
      case "done":
        this.complete = true;
        return;
      default:
        throw new Error(`Unknown trajectory event ${event.type}`);
    }
  }

  snapshot() {
    return [...this.scopes.values()]
      .filter(({ scope }) => scope.owner_span_id === null)
      .sort((left, right) =>
        left.scope.root_span_id < right.scope.root_span_id ? -1 : 1
      )
      .map((scope) => this.assemble(scope, new Set()));
  }

  assemble(scope, visiting) {
    const key = scopeKey(scope.scope);
    if (visiting.has(key)) throw new Error("Cycle in trajectory scopes");
    visiting.add(key);
    const turns = [...scope.turns.values()]
      .sort(
        (left, right) =>
          compareInstants(left.turn.start_time, right.turn.start_time) ||
          left.order - right.order
      )
      .map(({ turn, work }) => ({
        ...turn,
        work: [...work.entries()]
          .sort(([left], [right]) => left - right)
          .map(([, step]) => {
            const child = this.scopes.get(
              scopeKey({
                root_span_id: scope.scope.root_span_id,
                owner_span_id: step.id,
              })
            );
            return child
              ? { ...step, sub_agent: this.assemble(child, visiting) }
              : step;
          }),
      }));
    visiting.delete(key);
    return { ...scope.header, turns: [...scope.header.turns, ...turns] };
  }
}

export function runTrajectory(wasm, workload, validateCollector = false) {
  const start = performance.now();
  let stream;
  let oracle;
  try {
    stream = new wasm.TrajectoryStream(
      workload.spans.map(({ input: _input, output: _output, ...header }) =>
        wasm.import_span(header, true)
      ),
      false
    );
    const collector = new BrowserCollector();
    if (validateCollector) oracle = new wasm.TrajectoryCollector();
    let firstTurnMs;
    let events = 0;
    let snapshots = 0;
    let loaded = 0;
    const push = (batch) => {
      for (const event of batch) {
        if (event.type === "turn" && firstTurnMs === undefined)
          firstTurnMs = performance.now() - start;
        collector.push(event);
        oracle?.push(event);
        events++;
      }
    };
    const snapshot = () => {
      const output = collector.snapshot();
      if (oracle)
        assert.deepEqual(
          JSON.parse(JSON.stringify(output)),
          oracle.snapshot(),
          workload.name
        );
      snapshots++;
      return output;
    };
    for (;;) {
      const ids = stream.pendingIds(workload.batchSize);
      if (!ids.length) break;
      for (const id of ids) {
        const span = workload.byId.get(id);
        if (!span) throw new Error(`Missing trajectory payload ${id}`);
        if (++loaded > workload.spans.length)
          throw new Error("Trajectory requested too many payloads");
        push(stream.push(wasm.import_span(span, true)));
      }
      snapshot();
    }
    push(stream.finish());
    if (!collector.complete)
      throw new Error("Trajectory stream ended before completion");
    return { output: snapshot(), firstTurnMs, events, snapshots };
  } finally {
    oracle?.free();
    stream?.free();
  }
}

function text(content) {
  if (typeof content === "string") return content;
  return (content ?? [])
    .filter((part) => part.type === "text")
    .map((part) => part.text)
    .join("");
}

export function validateTrajectory(output, fixture) {
  assert.equal(output.length, 1);
  assert.deepEqual(
    output[0].turns.map((turn) => ({
      request_id: turn.request_id,
      response_id: turn.response_id ?? null,
      response_text: text(turn.response?.response),
      work_ids: turn.work.map((step) => step.id),
      user_texts: (turn.request ?? [])
        .filter((message) => message.role === "user")
        .map((message) => text(message.content)),
    })),
    fixture.turns
  );
  if (fixture.worker_responses) {
    assert.deepEqual(
      output[0].turns.flatMap((turn) =>
        turn.work.flatMap(
          (step) =>
            step.sub_agent?.turns.map((child) => child.response_id) ?? []
        )
      ),
      fixture.worker_responses
    );
  }
}
