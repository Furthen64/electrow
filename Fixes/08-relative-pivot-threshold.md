# Fix 08 — Use a relative pivot threshold in LU solve

Status: pending
Severity: medium (false errors on high-value components)

## Problem
`lu_solve()` treats any pivot with absolute magnitude `< 1e-12` as singular. A resistor of 10 GΩ has conductance 1e-13, so legitimate high-impedance networks are rejected as `SingularMatrix`. The fixed threshold also behaves inconsistently across circuits with very different scales.

## Evidence
- `engine/src/solver.rs:319` — absolute `1e-12` cutoff.
- `engine/src/solver.rs:302-309` — only checks finiteness, not scaling.

## Fix
Select the pivot threshold relative to the matrix scale, e.g. `epsilon * max_abs_entry` (with a small absolute floor for a fully zero matrix), where `epsilon` is a documented constant. Document the choice in `Docs/solver-algorithms.md` §4. Ensure the existing singular-matrix test still fails as expected.

## Acceptance
- A divider of two 100 GΩ resistors solves to the analytic midpoint without `SingularMatrix`.
- `singular_matrix_is_diagnosed` and floating-node tests still pass.
- `cargo test --workspace` passes.
