# Fix 14 — Report zero-length two-terminal elements as unsupported

Status: complete (2026-11-26). `post_points()` now rejects coincident endpoints for all two-terminal kinds as a `GeometryFailure`; ground's single post is unaffected. Added text and XML regression tests; 88 tests pass.
Severity: low-medium (silent electrical short)

## Problem
`post_points()` rejects degenerate geometry only for BJT/MOSFET. For `r`, `c`, `l`, `v`, `i`, `s`, `w`, zero length yields two identical posts; `node_for()` maps both terminals to the same node, silently shorting the element. `Docs/circuit-format.md:36` explicitly claims zero-length elements are reported as unsupported and never silently substituted.

## Evidence
- `engine/src/import_common.rs:120` — default `_ => Ok(vec![(x1,y1),(x2,y2)])` with no degeneracy check.
- `engine/src/import_common.rs:101-115` — BJT/MOSFET do check.
- `Docs/circuit-format.md:36` — documented claim.

## Fix
In `post_points()`, apply the degeneracy check to all two-terminal kinds (skip elements that legitimately have a single post, e.g. ground). Return a `GeometryFailure` reason such as `"zero-length <kind>"` so the record lands in the `unsupported` list with line context. Keep wire merging semantics for genuinely zero-length wires if the reference allows them — decide and document explicitly.

## Acceptance
- A dump with `r 0 0 0 0 0 1000` reports an unsupported record and does not create a self-shorted resistor.
- BJT/MOSFET zero-length behavior unchanged.
- `cargo test --workspace` passes.
