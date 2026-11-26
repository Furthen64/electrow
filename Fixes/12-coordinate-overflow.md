# Fix 12 — Guard coordinate arithmetic overflow in post geometry

Status: pending
Severity: medium (panic / wrong posts on crafted input)

## Problem
`interp_point2`, `bjt_posts` and `mosfet_posts` compute `gx*gx + gy*gy` and `dx*dx + dy*dy` in `i64` before casting to `f64`. Since coordinate tokens parse as `i64`, large values overflow: debug builds panic on arithmetic overflow, release builds wrap and produce wrong posts. Importers do not enforce a coordinate magnitude cap.

## Evidence
- `engine/src/import_common.rs:37,56,73` — `i64` multiplication before `as f64`.
- `engine/src/falstad/parser.rs:130-141` — `parse::<i64>()` accepts values up to ~9.2e18.

## Fix
Cast operands to `f64` before multiplying/summing (`(gx as f64)*(gx as f64) + ...`), or use `i128` for the intermediate. Additionally enforce a documented coordinate bound (e.g. ±1e6, matching the JSON validator) on all import paths and report out-of-range records explicitly.

## Acceptance
- A dump with a coordinate near `i64::MAX` returns an error/unsupported record, never panics or wraps.
- BJT/MOSFET post geometry tests still pass.
- `cargo test --workspace` passes.
