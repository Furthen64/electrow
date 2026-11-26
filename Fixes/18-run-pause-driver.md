# Fix 18 — Make run/pause actually advance simulated time

Status: complete (2026-11-26). `EngineClient` now runs a daemon driver thread (16 ms tick, 4 steps of 5e-6 s per tick) while `running`; `web/app.js` polls snapshots every 50 ms and stops on pause/reset. Added `test_run_driver` to the smoke suite. Docs updated with the chosen cadence (architecture §1/§2). Smoke test OK.
Severity: medium (documented feature is a no-op)

## Problem
`run`/`pause` only flip `self.running`. No timer exists in `backend/` or `web/` (`app.js` has no rAF/setInterval), so simulated time advances only on explicit `step` clicks. `Docs/architecture.md` claims a 16 ms display cadence and snapshot polling via `GET /api/snapshot`, neither of which is implemented.

## Evidence
- `engine/src/lib.rs:464-467` — `run`/`pause` set a boolean only.
- `web/app.js:86-90` — buttons bound to commands; no time loop.
- `backend/server.py:73-80` — `/api/snapshot` exists but is not polled by the UI.

## Fix
Decide the ownership boundary (architecture says Python owns session cadence): add a bounded background driver in the backend that, while `running`, issues periodic `step` commands at a configurable simulated-time-per-wall-second rate, coalesces snapshots, and stops on pause/disconnect. Keep simulated time independent of wall clock per the contract. Update `web/app.js` to poll/receive snapshots and cancel on pause. Document the chosen cadence.

## Acceptance
- Clicking `run` advances simulated time without manual `step`; `pause` stops it.
- No unbounded queues or runaway stepping on disconnect.
- Docs describe the implemented behavior.
- `python tests/smoke.py` and `cargo test --workspace` pass.
