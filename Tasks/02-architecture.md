# Task 02 — Architecture and interface contracts

Status: complete (2026-11-26). Depends on 01. Outputs: Docs/architecture.md, Docs/interface-schema.json and the versioned Rust/Python/web scaffold; verification recorded in Docs/architecture.md §9.

Rust owns circuit topology, component parameters and dynamic state, matrix assembly, solving, simulation time and measurement samples. Python owns local HTTP hosting, sessions, file operations and transport. Browser HTML/CSS/vanilla JavaScript owns schematic presentation, pointer/keyboard interactions and display of simulation snapshots. No circuit solving in Python or JS.

Proposed layout: engine/ (Cargo crate and tests), backend/ (Python package), web/ (static HTML/CSS/JS), tests/fixtures/, docs/. Keep reference folders outside runtime packaging.

Select and document the Rust/Python boundary after a small interoperability spike. Candidate: PyO3/maturin extension exposing coarse-grained engine operations; alternative: persistent Rust worker process with framed messages. Do not implement both. Consider GIL release, cancellation, crashes, build complexity and data-copy costs. Choose minimal Python HTTP framework with WebSocket support if needed; no heavy frontend framework by default.

Define versioned commands for create/load circuit, topology edit, parameter edit, reset, run/pause, step and measurement subscriptions. Define stable IDs, revisions, units, errors, snapshots with simulation time and sequence number. Engine applies edits at safe boundaries; reject stale revisions. Distinguish wall-clock UI cadence from simulated time. Bound queues and sample buffers; separate reliable edits/errors from replaceable frames.

Acceptance: written ownership diagram, API examples and schemas, dependency choices and rationale, minimal Rust/Python/browser round trip verified. No duplicate solver state.
