# Task 03 numerical validation (initial)

Status: Task 03 gate passed (2026-11-26). This is the first slice of [Task 08](../Tasks/08-validation.md); broader fixture generation and reference trace capture remain in Task 08.

## Fixture provenance

All current fixtures are independently authored analytic circuits in `engine/src/solver.rs` tests. No code or golden waveform was copied from `ref_SEVA77/`, `ref_Vactrol/` or `ref_LVLabs/`. Reference fixtures identified for later comparison are recorded in [reference-analysis.md](reference-analysis.md) §9 (`auto-tests/capac.txt`, `lrc.txt`, `fullrect-sc.txt`, `ota-vca.txt`, etc.); they are not imported or redistributed here.

Reference snapshots used for algorithm mapping: Vactrol upstream 4.1.4js (commit `c0b264e462fb8935c09b0e2a4dfa884debbde6b5`), LVLabs 4.1.3js, SEVA77 1.3.2 / 3.1.3js. Reference executables were not run in this slice, so no reference traces are claimed.

## Results

Command: `cargo test --workspace` → **19 passed, 0 failed** (11 solver, 6 protocol/state, 2 JSON).

| Check | Analytic expectation | Tolerance | Result |
|---|---|---:|---|
| `stamps_matrix_and_solves_known_system` | resistor + MNA voltage source, `V=[10,10]`, source current 0 | 1e-9 V/A | pass |
| `singular_matrix_is_diagnosed` | rank-deficient matrix rejected | exact diagnostic | pass |
| `resistive_divider_matches_analytic_result` | 10 V across two 1 kΩ resistors → midpoint 5 V, current 5 mA | 1e-9 V / 1e-9 A | pass |
| `rc_transient_matches_analytic_time_constant` | 10 V, 1 kΩ, 1 µF, 200 × 5 µs = 1 τ → `10(1-e^-1) ≈ 6.321 V` | 0.05 V | pass |
| `rl_transient_matches_analytic_time_constant` | 10 V, 100 Ω, 0.1 H, 1 τ → `(10/100)(1-e^-1) ≈ 63.2 mA` | 5 mA | pass |
| `diode_converges_to_shockley_operating_point` | 5 V, 1 kΩ, `Is=1e-12`, `Vt=0.025865` → `Vd ≈ 0.576 V`, `Id ≈ 4.42 mA` | 0.02 V / 0.2 mA | pass |
| `floating_nodes_and_singular_matrices_are_reported` | resistor island reports `FloatingNode`, solve reports `SingularMatrix` | exact diagnostic kinds | pass |
| `invalid_parameters_are_rejected` | zero resistance / unknown element rejected at edit time | exact diagnostic kind | pass |
| `rejected_step_restores_previous_accepted_state` | forced `NonConvergence` leaves state and time unchanged | exact equality | pass |
| `repeated_runs_are_deterministic` | identical circuits produce identical node voltages and time | exact equality | pass |
| `wires_merge_nodes` | wire merges `n2`/`n3` into one node | exact node list | pass |

Integration check: `python tests/smoke.py` drives the real Rust worker through the HTTP API, builds a 10 V divider over the protocol, steps once and verifies the midpoint is 5 V within 1e-6 V, the subscribed measurement matches, simulated time is 5 µs, and a disconnected resistor reports `floating_node`. Result: `SMOKE OK`.

## Tolerance rationale

- Divider/stamps use 1e-9 because the linear solve is deterministic and the values are exact rationals in f64.
- RC/RL use 0.05 V / 5 mA because trapezoidal integration at `dt/τ = 0.005` introduces a small discretization error relative to the continuous analytic result; this is a discretization comparison, not a reference-trace comparison.
- Diode uses 0.02 V / 0.2 mA because the Shockley operating point is solved by Newton iteration with a 0.01 V stopping rule and a 0.1 V step limit.
- Diagnostics and determinism are compared exactly because they are discrete outcomes.

## Deliberate deviations from reference behavior

Documented in [solver-algorithms.md](solver-algorithms.md) §5–8: trapezoidal only (no backward Euler option), no `gmin` ramp, 0.1 V Newton step limit, no `1e8` floating-node resistor, dense-only solve, diode breakdown not implemented. None of these alter the analytic fixtures above; they become reference-comparison items in Task 08.

## Not yet validated

- Golden waveform comparison against donor `auto-tests/` fixtures (Task 08).
- KCL residual checks, frequency/settling checks, import/export round trips, reconnection and browser smoke tests (Tasks 05–08).
- Performance measurements (Task 09).
