# Emit base64 Anthropic PDF document sources

The universal-file converter emits source.type=text for all inline documents, including application/pdf. Anthropic's canonical schema requires source.type=base64 for PDF data. URL sources and text/plain documents must retain their existing representations.

Update crates/lingua/src/providers/anthropic/convert.rs to choose Base64 for inline PDFs. Extend the router PDF payload regression to assert the source discriminator for Anthropic and Vertex Anthropic. Correct the converter's PDF expectation and add explicit text/plain preservation coverage.

The mapping is a direct representation of PDF bytes; no schema generation or provider-managed semantics change is needed. No expected-difference exceptions are intended.

Validation: reproduce the failing router payload test; attempt make capture FILTER=pdfDocumentBase64SourceParam; run focused Anthropic and router tests; run payload/cross-provider/typed-boundary checks where the worktree environment supports them. Do not retain invalid live-capture responses as expected fixtures.

The router regression reproduced source.type=text before the fix and passes after it for both Anthropic and Vertex Anthropic. All 35 Anthropic converter tests pass, including inline PDF, text/plain, and URL documents. The cross-provider coverage check and typed-boundary checks pass. The temporary capture case was removed after the initial capture was blocked by the worktree tooling setup; no response snapshots were fabricated or changed. After installing locked dependencies and rebuilding WASM, all 2,616 payload/sync tests passed (185 skipped). The test rerun required permission to bind local mock-server ports. Live provider responses were not verified in this follow-up.
