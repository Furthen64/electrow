# Fix 17 — Make serialization reject non-finite numbers symmetrically

Status: complete (2026-11-26). Added `Value::has_non_finite()`; `export_circuit` and the worker's response writer now emit protocol errors instead of `null` for non-finite values. Added `detects_non_finite_values`; 91 tests pass; smoke test OK.
Severity: low (export/re-import asymmetry)

## Problem
Validation rejects non-finite parameter/option values, but `write_value()` serializes any non-finite `f64` as `null`. A programmatically built document with NaN/inf therefore exports JSON that will be rejected on re-import, with no indication of the problem.

## Evidence
- `engine/src/json.rs:252-257` — `Value::Number` serialization emits `null` for non-finite values.
- `engine/src/circuit_doc/serialize.rs:7-13` — delegates to `write_value`.

## Fix
Make serialization explicit: either reject non-finite numbers by returning an error from `to_json()`/`document_to_json`, or emit a documented diagnostic alongside the export. Prefer failing loudly so a bad document cannot be written out and silently fail later.

## Acceptance
- Serializing a document containing a non-finite value returns an error/diagnostic rather than `null`.
- All existing export round-trip tests still pass.
- `cargo test --workspace` passes.
