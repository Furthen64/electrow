# Fix 04 — Connect imported ground symbols to their coordinate post

Status: pending
Severity: high (imported circuits float / go singular)

## Problem
`assign_nodes()` special-cases `ground` to `["gnd"]` without calling `node_for()` on its post coordinate. Any wire or component endpoint drawn exactly on the ground symbol's post gets a fresh `nK` node, leaving the ground element isolated. Reproduced: wire/source endpoints at the ground post became `n1`/`n2` while ground stayed `gnd`.

## Evidence
- `engine/src/import_common.rs:119` — `post_points()` does return the ground post `(x1, y1)`.
- `engine/src/import_common.rs:160-165` — ground never registered in the coordinate→node map.

## Fix
Register the ground post before assigning nodes: for a ground element call `node_for(post, ...)` (or explicitly insert its coordinate → `"gnd"` into `node_ids` before the general loop), so any endpoint sharing that coordinate resolves to `"gnd"`. Apply the same rule in both text and XML importers (they share this pipeline).

## Acceptance
- A dump with `g`, a wire endpoint and a voltage-source terminal all on the same coordinate imports with all three on node `"gnd"`.
- Equivalent XML fixture produces the same connectivity (existing text/XML comparison tests still pass).
- `cargo test --workspace` passes.
