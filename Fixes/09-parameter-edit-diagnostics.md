# Fix 09 — Refresh diagnostics after parameter edits

Status: pending
Severity: low-medium (stale diagnostics)

## Problem
`Element::validate()` is run inside `analyze()`, but `set_parameter()` only mutates the parameter and calls `validate()` on that element, discarding the result. `self.diagnostics` therefore keeps pre-edit values until a topology edit or reset, and a newly invalid parameter is returned as an error but never appears in snapshots.

## Evidence
- `engine/src/solver.rs:510-524` — `set_parameter()` does not call `analyze()`.
- `engine/src/solver.rs:587-592` — `analyze()` rebuilds diagnostics.

## Fix
After a successful parameter edit, re-run the diagnostic scan (a lightweight `refresh_diagnostics()` that validates elements and recomputes ground/floating-node state) without re-doing full topology rebuild if unnecessary. Coordinate with Fix 05 so diagnostics stay bounded.

## Acceptance
- Editing a parameter back to a valid value clears the corresponding diagnostic from the next snapshot.
- Editing to an invalid value surfaces the diagnostic consistently.
- `cargo test --workspace` passes.
