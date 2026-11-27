# Agent instructions

## Temporary work

Use the `Temp/` directory at the repository root for all temporary files: scratch scripts, generated reports, intermediate data, downloaded artifacts and build experiments.

- Write temporary output to `Temp/`, not to the repository root or other project directories.
- `Temp/` is gitignored; do not force-add its contents.
- Keep `ref_SEVA77/`, `ref_Vactrol/` and `ref_LVLabs/` read-only; never place temporary copies of reference material outside `Temp/`.
- Delete scratch files when they are no longer needed; keep harness output under `Temp/` rather than the repo root.

## Verification

Run these before reporting implementation work as complete. All commands run from the repository root.

- `cargo test --workspace` — Rust unit/integration tests (engine solver, imports, protocol).
- `python tests/smoke.py` — end-to-end HTTP + real Rust worker, versioned protocol, solver round trip, file endpoints and error mapping.
- `python -m backend` — launches the local UI on `http://127.0.0.1:8000/` for manual testing.
- `python tests/test_api.py` — Python API tests for the real worker and HTTP server (revisions, errors, subscriptions, host checks).
- `python tests/boundary_bench.py` — measures Python↔Rust command round-trip overhead and transport bytes; use when changing the worker protocol or transport.
- `cargo run --release --example perf_bench -- --out Temp/perf.json` — release performance harness covering small, passive, nonlinear and topology-edit workloads; use before/after solver changes.
- `./tests/run_browser_ui.sh` — browser UI harness. Starts a local backend (port 8123) and headless Chrome with CDP (port 9222), runs `tests/browser_ui.cjs`, and tears both down. Requires Chrome and Node 22+ (global `WebSocket`). Screenshots and logs are written to `Temp/browser-ui/`. Override with `ELECTROW_CHROME`, `ELECTROW_BROWSER_PORT` and `ELECTROW_CDP_PORT`.
- `node tests/browser_ui.cjs --base <url> --cdp <url> --out <dir>` — the same browser checks against already-running services; no servers are started or stopped.

The browser harness verifies the schematic editor as a user would: divider demo (midpoint 5 V), Run/scope, palette placement, undo/redo, save/load and Falstad import. Do not add frontend solver logic to make checks pass; browser code displays engine snapshots only.
