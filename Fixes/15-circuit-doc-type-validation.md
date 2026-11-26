# Fix 15 — Reject wrong-typed params/options in circuit_doc

Status: complete (2026-11-26). `validate.rs` now rejects wrong-typed `params`/`options`/`geometry`, non-boolean flags, negative/oversized integer flags and wrong-typed booleans instead of silently defaulting. Added `rejects_wrong_typed_fields`; 89 tests pass.
Severity: low (malformed input silently accepted)

## Problem
`validate.rs` and `parse.rs` use `unwrap_or(default)` / `if let` when reading `params` and `options`, so `"params": 5` or `"max_time_step_s": "abc"` produce no diagnostic and are silently replaced. Integer flags are only checked with `fract() == 0`, so negative or huge values pass and are then truncated/zeroed by `as_u64()`/`as u32`.

## Evidence
- `engine/src/circuit_doc/validate.rs:85-86,99,186-212` — defaults on wrong type.
- `engine/src/circuit_doc/parse.rs:67-91,104-111,123-126` — same pattern and integer truncation.

## Fix
Add type checks in validation: `params` and `options` must be objects; each known option must be a finite number in range; flags must be finite non-negative integers within `u32`/`u64` range. Emit a `DocDiagnostic` with the field path for every mismatch instead of defaulting. Keep defaults only for genuinely absent optional fields.

## Acceptance
- Documents with wrong-typed `params`/`options` or invalid flags are rejected with clear diagnostics.
- Valid documents, including omitted optional fields, still round-trip.
- `cargo test --workspace` passes.
