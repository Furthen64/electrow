# Fix 21 — Align interface schema and docs with implementation

Status: complete (2026-11-26). Schema now covers all four load formats, `export_circuit`, emitted error codes and the real status enum; stale claims in `architecture.md`, `solver-algorithms.md`, `circuit-format.md` and `falstad/mod.rs` corrected. Error envelopes deliberately omit `revision` (documented) because errors can precede circuit creation.
Severity: low (contract drift)

## Problem
`Docs/interface-schema.json:104` allows only `["text","xml"]` for `load_circuit.format`, but the engine accepts `electrow-json`, `falstad-text`, `text`, `xml`. `export_circuit` and the emitted codes `bad_document`/`invalid_document`/`no_document` are absent from the schema; `status: "converging"` and `solver_failed` are declared but never emitted. `fail()` omits `revision` despite the architecture doc saying edits/errors always carry it. `falstad/mod.rs:14-17` still claims BJT/MOSFET import is unsupported, and `Docs/circuit-format.md:3` claims a `geometry` module that does not exist.

## Evidence
- `Docs/interface-schema.json:29-41,104,296-309,334`
- `engine/src/lib.rs:70-84,235-237,166`
- `Docs/architecture.md:94,147,209,216,224`
- `engine/src/falstad/mod.rs:14-17`

## Fix
Regenerate/update the schema from the actual command surface and diagnostic enum; add `revision` to error envelopes; remove stale/unused enum values. Update stale doc claims (BJT/MOSFET support, geometry module location, test counts, Task 05 status, snapshot pull behavior). Treat this task as the final documentation pass after Fixes 18–20 settle the runtime behavior.

## Acceptance
- Schema validates real request/response samples for every command.
- No doc statement contradicts current code.
- Reviewers can trace each emitted code to the schema.
