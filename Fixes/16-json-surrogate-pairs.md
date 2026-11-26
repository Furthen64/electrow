# Fix 16 — Combine UTF-16 surrogate pairs in JSON strings

Status: complete (2026-11-26). `\u` escapes now combine high+low surrogates into the non-BMP code point and reject unpaired surrogates. Added `combines_surrogate_pairs_and_rejects_lone_surrogates`; 90 tests pass.
Severity: low (data corruption)

## Problem
`string()` replaces every code unit in `0xd800..=0xdfff` with U+FFFD, so valid JSON escapes like `\uD83D\uDE00` corrupt non-BMP characters in ids, titles and dump strings.

## Evidence
- `engine/src/json.rs:155-163` — surrogate handling in the `\u` escape branch.

## Fix
When a high surrogate (`0xd800..=0xdbff`) is parsed, require a following `\u` low surrogate (`0xdc00..=0xdfff`), combine into the code point `0x10000 + ((hi-0xd800)<<10) + (lo-0xdc00)`, and emit that `char`. Reject lone surrogates as a parse error (or U+FFFD with a documented warning) and reject low surrogates without a high surrogate.

## Acceptance
- `json::parse(r#"{"s":"\uD83D\uDE00"}"#)` yields a string containing U+1F600.
- Lone surrogates are rejected or explicitly reported.
- `cargo test --workspace` passes.
