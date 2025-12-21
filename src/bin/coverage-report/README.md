# Coverage report

Cross-provider transformation coverage for requests and responses.

## Run

```bash
cargo run --bin coverage-report
```

## Output

- Request transformation matrix (NxN providers)
- Response transformation matrix (NxN providers)
- Summary with pass/fail counts
- Collapsible list of all failures with error messages

## Add a provider

1. **Add to PROVIDERS** in `main.rs`:
   ```rust
   ("dir-name", "DisplayName", ProviderFormat::Xxx),
   ```

2. **Capture payloads** for each test case (see `payloads/README.md`):
   ```
   payloads/snapshots/{test-case}/{dir-name}/
     request.json
     response.json
     followup-request.json  (optional)
   ```

3. **Run report** to verify. Missing payloads show as failures: `Source payload not found`
