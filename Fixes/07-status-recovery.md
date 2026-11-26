# Fix 07 — Clear a failed status on later success

Status: pending
Severity: medium (misleading UI/snapshot state)

## Problem
`Engine::step()` sets `status = "failed"` when a step fails, but no later successful step, edit, or run restores `"ready"`. Snapshots keep reporting `"failed"` indefinitely until an explicit `reset`, edit or run command.

## Evidence
- `engine/src/lib.rs:485-490` — failure sets status; success path does not reset it.
- `engine/src/lib.rs:464-467` — `run`/`pause` overwrite status, so the state is inconsistent depending on command order.

## Fix
After `self.circuit.step(...)`, set status to `"ready"` (or `"running"` when `self.running`) on success and `"failed"` only on failure. Centralize status derivation from `(running, last_step_failed)` so every command path produces a consistent value.

## Acceptance
- Failed step → snapshot `status == "failed"`; subsequent successful step → `status` recovers without `reset`.
- `cargo test --workspace` passes.
