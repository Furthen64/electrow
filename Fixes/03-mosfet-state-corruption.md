# Fix 03 — Store gate voltage in MOSFET state

Status: pending
Severity: high (wrong simulation results for non-ideal gate drive)

## Problem
`commit()` writes `entry.v = drain − source`, but `assemble()` and `attempt_step()` treat `state.v` as the gate voltage (`vs[0]`). After the first committed step the state field holds Vds instead of Vgs, so every later step starts Newton from a wrong linearization point. Reproduced: stored `v=3.0` when gate = 5 V and Vds = 3 V. Existing test passes only because gate and drain are ideal sources.

## Evidence
- `engine/src/solver.rs:1045` — `previous.v = vs[0]` (gate voltage).
- `engine/src/solver.rs:825` — `vs = [previous.v, previous.v2, previous.v3]` read as gate/source/drain.
- `engine/src/solver.rs:940` — `entry.v = vs[2] - vs[1]` (drain − source) breaks the convention.

## Fix
Set `entry.v = vs[0]` in the MOSFET branch of `commit()` (or rename/restructure the state fields so the meaning is unambiguous and shared by all three sites). Add a comment only if needed to document the convention; prefer a shared helper/constant used by assemble, iterate and commit.

## Acceptance
- A test asserts `state.elements["m1"].v` equals the gate voltage after stepping.
- A circuit with the gate driven through a resistor/capacitor converges to the expected operating point across multiple steps.
- `cargo test --workspace` passes.
