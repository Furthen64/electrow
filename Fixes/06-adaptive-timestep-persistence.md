# Fix 06 — Preserve adaptive timestep across step calls

Status: complete (2026-11-26). Added `started` flag; `dt_s` now seeds `current_dt` only on the first step after create/reset, otherwise adaptation persists. `reset()` restores the default dt. Documented in `solver-algorithms.md` §6 and `interface-schema.json`. Added `adaptive_dt_persists_across_step_calls`; 79 tests pass.
Severity: medium (performance / behavior)

## Problem
`Circuit::step()` overwrites `current_dt` with the requested `dt` at the start of every call. The 3-good-step growth logic then only applies within a single multi-step call. Normal backend usage (one `step` command per request) restarts at the requested dt every time, so adaptation never persists.

## Evidence
- `engine/src/solver.rs:1073-1075` — `self.current_dt = requested_dt.clamp(...)` at entry.
- `engine/src/solver.rs:1109-1113` — growth after good steps, but not carried across calls.

## Fix
Treat `requested_dt` as the initial value only when no prior step has been taken (or as an explicit reset via `reset()` / a new `set_base_dt` command). Otherwise keep `self.current_dt` and clamp only to `[min_dt_s, max_dt_s]`. Document the chosen semantics in `Docs/solver-algorithms.md` §6 and `Docs/interface-schema.json`.

## Acceptance
- Calling `step(dt, 1)` three times grows dt on the third call, matching one `step(dt, 3)` call.
- `reset()` restores the initial requested/default dt.
- `cargo test --workspace` passes.
