# Fix 02 — Bound JSON parser depth and string scan cost

Status: pending
Severity: high (CPU/memory DoS on untrusted input)

## Problem
Two issues in the hand-written JSON parser:
- `string()` revalidates the entire remaining byte slice with `from_utf8` for every character, giving O(n²) parsing. Measured: 40 KB string → 49 ms; a ~1 MiB request would take tens of seconds.
- `value()` ↔ `object()`/`array()` recurse with no depth limit; a ~1 MiB request of nested `[`/`{` overflows the Rust stack and aborts the worker.

## Evidence
- `engine/src/json.rs:170` — `std::str::from_utf8(&self.bytes[start..])` per character.
- `engine/src/json.rs:84,195,217` — unbounded recursive descent.

## Fix
- Decode one UTF-8 char in place using `self.bytes[self.pos]` length via `utf8_char_width` / a leading-byte check and `std::str::from_utf8(&self.bytes[pos..pos+len])`, advancing `pos` by exactly that char's length. Never slice to the end of the buffer.
- Add a `MAX_DEPTH` constant (e.g. 128) and a depth counter threaded through `value()`/`object()`/`array()`; return an error above the limit.
- Do not use a `serde` dependency (crate is dependency-free by design).

## Acceptance
- Parsing a 1 MiB string completes in linear time (< ~100 ms on the review machine).
- A 1 MiB deeply nested input returns a parse error without crashing.
- Regression tests added; `cargo test --workspace` passes.
