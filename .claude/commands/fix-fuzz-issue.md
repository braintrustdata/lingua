# Fix a fuzz roundtrip issue

## 1. Run the fuzzer

Run stats mode to see the full scope of issues:

```
make -C crates/lingua/tests/fuzz stats
```

Run fail-fast mode to get a minimal repro with verbose output:

```
make -C crates/lingua/tests/fuzz run
```

Pick one issue from the output.

## 2. Verify with a real request

Confirm the input payload is valid by sending it to the real provider API:

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '<minimal failing input JSON>'
```

If the provider rejects it, the issue may be invalid fuzzer output. Check the OpenAPI spec in `specs/` to confirm.

## 3. Write a failing unit test

Add a test in the appropriate module following existing patterns. Keep it minimal.

```bash
cargo test -p lingua <test_name>
```

Confirm it fails.

## 4. Fix and verify

Make the fix. Then:

```bash
cargo test -p lingua <test_name>
cargo test -p lingua
make -C crates/lingua/tests/fuzz stats
```

## 5. Commit

Separate commits for test infrastructure changes vs the bugfix.
