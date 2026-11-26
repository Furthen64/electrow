# Task 06 — Python local backend

Status: pending. Depends on 02, 03, 05.

Serve static web assets and versioned API from a local Python application. Use the chosen Rust bridge; do not reimplement solver logic. Bind to loopback by default, validate Origin/Host where appropriate, limit request sizes and avoid arbitrary filesystem access. Remote hosting is not part of MVP.

Implement session lifecycle, circuit load/save, validated edit commands, start/pause/reset/step controls, diagnostics and measurement subscriptions. Use HTTP for request/response operations and a WebSocket or other documented stream for live snapshots. Batch solver steps and samples at the boundary; never invoke Python per matrix stamp or per component. Ensure only one owner mutates each simulation session; isolate blocking work from server event loop and release GIL if using native bindings.

Bound queues, coalesce obsolete display snapshots, preserve command ordering and revision IDs. Handle disconnect/reconnect, invalid edits, solver failure and graceful shutdown. Separate simulated time from browser frame rate.

Acceptance: local server launches, serves UI, drives Rust engine through API, returns errors without crashing, reconnects correctly and shuts down cleanly. Integration tests use real Rust solver and small circuits; document run instructions and measured boundary overhead.
