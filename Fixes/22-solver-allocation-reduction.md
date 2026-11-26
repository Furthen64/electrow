# Fix 22 — Reduce solver allocations and linear lookups (measure first)

Status: pending
Severity: low-medium (performance; Task 09 territory)

## Problem
The solver allocates aggressively on the hot path:
- `Matrix::new` builds a fresh `vec![vec![0.0; size]; size]` every Newton iteration and every step; `lu_solve` copies it again with `matrix.to_vec()`.
- `attempt_step` does `self.elements.iter().find()` for each nonlinear element on every iteration.
- `step` clones the whole `State` (and `commit` clones it again); `node_voltages()` clones every node id/string on each snapshot.

For dense MNA this is O(n³) per iteration with no structure reuse. This is acknowledged as deferred in `Docs/solver-algorithms.md` §4, so profile before optimizing.

## Evidence
- `engine/src/solver.rs:190-196,310,954,948,1078,968,376-386`

## Fix
Profile representative circuits first. Then, without changing numerical results: preallocate/reuse a matrix buffer across iterations, build a `HashMap<id, index>` (or iterate elements once) instead of repeated `find()`, and eliminate redundant `State` clones where a mutable borrow suffices. Keep dense-only for now; sparse strategy remains a separate Task 09 decision backed by measurements.

## Acceptance
- A reproducible benchmark harness records before/after timings on the same circuits.
- All numerical fixtures pass unchanged after each optimization.
- No performance claims without measured numbers.
