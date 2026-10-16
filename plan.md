# Bedrock document validation fixes

- Root cause: user File parts become document blocks without checking the same-message text requirement. Binary data URLs are mistaken for remote URIs.
- Target files: Bedrock convert.rs; payloads/cases/params.ts; generated captures and snapshots; case-specific coverage exceptions if needed.
- Expected behavior: reject document-only user messages explicitly; preserve text-plus-document requests; accept binary base64 data URLs only with a matching MIME type; continue rejecting remote URIs and malformed data URLs.
- Tests: document-only rejection (including text in a different message), binary data URL extraction, MIME mismatch and malformed data URL rejection. Update successful document fixtures to include text.
- Expected differences: new document-only PDF case inherits only the existing filename and Anthropic document metadata limitations. Bedrock remains an explicit unsupported mapping for document-only requests.
- Validation: baseline make capture FILTER=chatCompletionsDocumentOnlyPdfFileParam (Bedrock service rejects both modes); failing focused Rust tests; implement; focused Rust tests; make capture FILTER=chatCompletionsDocumentOnlyPdfFileParam; make test-payloads; regenerate failed artifacts only if necessary; cross-provider coverage test; make typed-boundary-check; make typed-boundary-check-branch BASE=main; formatting and TypeScript checks.
