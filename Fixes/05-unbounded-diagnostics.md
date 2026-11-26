# Fix 05 — Bound solver diagnostics growth

Status: pending
Severity: high (unbounded response growth → protocol desync)

## Problem
`Circuit::step()` pushes a diagnostic for every failed timestep halving into `self.diagnostics`. The vector is only cleared by `analyze()`/`reset()`, not by `step()` or `set_parameter()`. `snapshot()` serializes every entry. Reproduced: 182 diagnostics after 20 failed steps (~9/step); growth eventually exceeds the 4 MiB response limit in `engine_client.py`, which desynchronizes the NDJSON stream.

## Evidence
- `engine/src/solver.rs:1095` — push per failed attempt.
- `engine/src/solver.rs:506,639` — only `analyze()`/`reset()` clear.
- `engine/src/lib.rs:580` — snapshot serializes all diagnostics.
- `backend/engine_client.py:14,71-80` — 4 MiB read limit and no resync path.

## Fix
Decide and implement a bounded policy: e.g. keep only unique diagnostics per step (dedupe) and cap `self.diagnostics` at a small fixed maximum (e.g. 16) via a ring buffer / `truncate`, or clear at the start of each `step()` and report only that step's failures. Ensure `set_parameter()` also refreshes state (see Fix 09). Keep the deterministic failure diagnostics needed by tests.

## Acceptance
- 1000 failed steps produce at most the cap; snapshot size stays bounded.
- Existing tests checking for `SingularMatrix`/`NonConvergence`/`FloatingNode` still pass.
- `cargo test --workspace` passes.
