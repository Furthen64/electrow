# Task 06 — Python local backend

Status: complete (2026-11-26). Depends on 02, 03, 05. Delivered: loopback `ThreadingHTTPServer` with Host/Origin checks, 1 MiB request cap, size-limited worker I/O with a 5 s timeout and resync, serialized single-owner sessions, daemonized step driver (16 ms tick, 4 steps/tick) and browser snapshot polling (50 ms, stale frames dropped), bounded/deduplicated diagnostics, `session_lost` recovery, graceful shutdown, and circuit JSON save/load/list endpoints confined to `ELECTROW_CIRCUITS_DIR` with validated names. Docs: `architecture.md` §4–5.1 and `README.md` run instructions. Evidence: `python tests/smoke.py` (engine client, run driver, solver round trip, HTTP errors, circuit file round trip and path rejection) passes; `python tests/boundary_bench.py` measured snapshot 0.104 ms mean / 0.219 ms p95 and step 0.092 ms mean / 0.158 ms p95 per command.

Serve static web assets and versioned API from a local Python application. Use the chosen Rust bridge; do not reimplement solver logic. Bind to loopback by default, validate Origin/Host where appropriate, limit request sizes and avoid arbitrary filesystem access. Remote hosting is not part of MVP.

Implement session lifecycle, circuit load/save, validated edit commands, start/pause/reset/step controls, diagnostics and measurement subscriptions. Use HTTP for request/response operations and a WebSocket or other documented stream for live snapshots. Batch solver steps and samples at the boundary; never invoke Python per matrix stamp or per component. Ensure only one owner mutates each simulation session; isolate blocking work from server event loop and release GIL if using native bindings.

Bound queues, coalesce obsolete display snapshots, preserve command ordering and revision IDs. Handle disconnect/reconnect, invalid edits, solver failure and graceful shutdown. Separate simulated time from browser frame rate.

Acceptance: local server launches, serves UI, drives Rust engine through API, returns errors without crashing, reconnects correctly and shuts down cleanly. Integration tests use real Rust solver and small circuits; document run instructions and measured boundary overhead.
