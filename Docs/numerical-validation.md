# Task 08 — numerical and integration validation

Status: gate passed (2026-11-27). This document is the validation report for the MVP solver, imports, protocol, backend and browser editor.

## Fixture provenance

- All fixtures live in `tests/fixtures/` and carry `id`, `version: 1`, `source`, input document/dump, solver settings and explicit expected values with absolute/relative tolerances.
- Analytic fixtures are independently authored from closed-form solutions; no waveform was copied from `ref_SEVA77/`, `ref_Vactrol/` or `ref_LVLabs/`. Falstad-syntax fixtures use the supported dump grammar and are independently authored.
- Reference execution was **not available**: the three donor snapshots are Java/GWT source trees with no built binary or runnable harness in this repository. No donor traces are claimed and no golden data was invented. Reference versions used for algorithm mapping remain Vactrol `c0b264e462fb8935c09b0e2a4dfa884debbde6b5` (upstream 4.1.4js), LVLabs 4.1.3js and SEVA77 1.3.2 / 3.1.3js. Capturing donor traces is an open Task 08 follow-up if a donor runtime becomes available.

## Fixtures and results

| Fixture | Source | Check | Expected | Tolerance | Result |
|---|---|---|---:|---:|---|
| `01-divider-dc.json` | analytic | midpoint / current | 5 V / 5 mA | 1e-9 V / 1e-9 A | pass |
| `01-divider-dc.json` | analytic | KCL residual at n2 | 0 A | 1e-9 A | pass |
| `02-rc-step.json` | analytic | Vc at one τ (1 ms) | 6.3206 V | 0.05 V | pass |
| `03-rl-step.json` | analytic | I at one τ (1 ms) | 63.21 mA | 5 mA | pass |
| `04-rlc-settling.json` | analytic | first peak / settled Vc | 14–18 V / 10 V | range / 0.1 V | pass |
| `05-diode-forward.json` | analytic | Vd / Id | 0.576 V / 4.42 mA | 0.02 V / 0.2 mA | pass |
| `06-bjt-switching.json` | analytic | Vbe / Vc / Ic | 0.4–0.8 V / 0–10 V / > 0 | range | pass |
| `07-mosfet-linear.json` | analytic | Vgs / Vds / Ids | 5 V / 3 V / 0.12 A | 1e-9 / 1e-9 / 5 mA | pass |
| `08-falstad-divider.json` | supported dump | 5 elements, 0 unsupported, shared junction | exact | pass |
| `09-unsupported-import.json` | supported dump | 1 element, 1 unsupported `403` record | exact | pass |

Additional integration checks in `engine/tests/validation.rs`: document round trips for four fixtures, deterministic repeated runs, reset-to-zero state/time, rejected-step rollback with `NonConvergence`, topology-edit-induced floating nodes and singular solves, and parameter validation.

## Tolerance rationale

- Linear DC and ideal-source checks use 1e-9 because the dense solve is deterministic and values are exact rationals in f64.
- RC/RL use 0.05 V and 5 mA because trapezoidal integration at `dt/τ = 0.005` has a small discretization error versus the continuous analytic result.
- RLC uses a peak *range* (underdamped overshoot depends on the companion-model discretization) and a 0.1 V settling band after ten envelope time constants.
- Diode/BJT/MOSFET tolerances reflect the 0.01 V Newton stopping rule and 0.1 V step limit; MOSFET current uses a 5 mA band around the square-law result.
- Diagnostic, import and determinism checks are exact because they are discrete outcomes.

## Test counts and commands

- `cargo test --workspace`: **102 passed, 0 failed, 1 ignored benchmark** (91 unit + 11 fixture/integration tests).
- `python tests/smoke.py`: real Rust worker, versioned protocol, solver round trip, file endpoints, error mapping, run driver — **SMOKE OK**.
- `python tests/test_api.py`: 5 Python API/HTTP tests — **OK**.
- `python tests/boundary_bench.py`: measured worker overhead (`snapshot` 0.104 ms mean, `step` 0.092 ms mean on the development machine).
- `./tests/run_browser_ui.sh`: 13 browser checks against the real UI (divider 5 V, Run/scope, palette, undo/redo, file round trip, Falstad import) — all pass; screenshots in `Temp/browser-ui/`.
- `.github/workflows/ci.yml` runs the Rust, Python and browser jobs on push/PR and uploads browser screenshots.

## Deliberate deviations from the references

Documented in [solver-algorithms.md](solver-algorithms.md): trapezoidal integration only, no `gmin` ramp, 0.1 V Newton step limit, no hidden `1e8` Ω floating-node resistor, dense-only LU with a relative pivot threshold, diode breakdown not implemented. None of these change the analytic fixtures above; donor-trace comparison remains open. AC waveforms, controlled sources, op-amps and digital gates are not part of the MVP, so a true diode rectifier fixture is deferred rather than fabricated.

## Not yet validated

- Reference-trace comparison against donor `auto-tests/` fixtures when a donor runtime is available.
- KCL residual sweep across nonlinear and reactive circuits (currently checked on the DC divider fixture).
- Long-duration stability and frequency-domain checks beyond the RLC settling fixture.
