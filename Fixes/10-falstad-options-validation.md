# Fix 10 — Validate Falstad options on import

Status: pending
Severity: medium (invalid simulation settings)

## Problem
`parse_options_line()` accepts `minTimeStep` tokens without validation: NaN, inf, negatives and values greater than `maxTimeStep` enter the document unchecked. Unlike the `electrow-json` path, importer output is never passed through `circuit_doc::validate`.

## Evidence
- `engine/src/falstad/parser.rs:78-88` — unvalidated `minTimeStep`.
- `engine/src/falstad/parser.rs:89-94` — `voltageRange`/`maxTimeStep` do have finite checks (inconsistent).
- `Docs/circuit-format.md` — claims validated SI parameters.

## Fix
Apply the same validation rules as `circuit_doc::validate` to all imported options: finite, positive time steps, `min <= max`, finite positive voltage range. Record invalid values as a fatal `DocDiagnostic` or a documented warning with line context — never silently substitute a default.

## Acceptance
- Imports with `NaN`, `inf`, negative or inverted timesteps are rejected or warned explicitly.
- Valid fixtures still import identically.
- `cargo test --workspace` passes.
