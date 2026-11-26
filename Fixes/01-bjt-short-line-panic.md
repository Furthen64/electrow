# Fix 01 — Reject short BJT dump lines without panicking

Status: complete (2026-11-26). Changed `elements.rs:157` to use `raw.tokens.get(1..).unwrap_or(&[])`; audited all other token accesses (guarded by length checks). Added `falstad::tests::short_bjt_lines_are_reported_without_panicking`. `cargo test --workspace`: 71 passed, 0 failed.
Severity: high (worker crash on user input)

## Problem
A Falstad BJT line with geometry but no parameter tokens (`t x1 y1 x2 y2 flags`, exactly 6 tokens) passes the parser's token-count check, then `elements.rs` slices `&raw.tokens[1..]` on an empty vector and panics. Reproduced: `range start index 1 out of range for slice of length 0`. Untrusted input can kill the engine worker.

## Evidence
- `engine/src/falstad/parser.rs:124-129` — accepts `tokens.len() == 6`, stores `tokens[6..]` as empty.
- `engine/src/falstad/elements.rs:149-157` — `pnp_token` defaults to `"1"`, then `&raw.tokens[1..]` panics on empty input.

## Fix
Make the BJT parameter parse take `raw.tokens.get(1..).unwrap_or(&[])` (or explicitly reject an empty parameter tail with a `DocDiagnostic`/`UnsupportedRecord` before slicing). Keep the documented `[pnp]` short form either working safely or explicitly reported as unsupported. Audit other kind parsers for the same `tokens[n..]` pattern.

## Acceptance
- `load_circuit` with dump `"$ 1 5.0E-6 10 63 10.0 62\nt 0 0 100 100 0\n"` returns an error response, never panics.
- Regression test added; `cargo test --workspace` passes.
