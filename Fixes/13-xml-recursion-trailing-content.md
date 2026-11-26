# Fix 13 — Bound XML parser recursion and reject trailing content

Status: complete (2026-11-26). `parse_element()` now takes a depth capped at 64; `parse_document()` rejects trailing content after the root. Added `xml_rejects_deep_nesting_and_trailing_content`; 86 tests pass.
Severity: medium (stack overflow / silently accepted input)

## Problem
`parse_element()` recurses once per nested child with no depth limit; a deeply nested document can overflow the stack and abort the worker. `process_element()` rejects nested content only after the parser has already recursed. Separately, `parse_document()` parses the root but ignores any trailing content, so `<circuit>...</circuit><extra/>` is accepted as valid.

## Evidence
- `engine/src/xml_import.rs:148-238,229-231` — unbounded recursion.
- `engine/src/xml_import.rs:241-252` — no trailing-content check.

## Fix
Add a recursion depth parameter with a documented cap (e.g. 64) and return `DocDiagnostic` above it. After parsing the root, skip whitespace and require end-of-input; otherwise return a diagnostic quoting the trailing token. Keep within the existing document-size cap.

## Acceptance
- Deeply nested XML (< size cap) returns a parse error without crashing.
- Trailing tags/text after `</circuit>` are rejected.
- Existing XML fixtures still pass.
- `cargo test --workspace` passes.
