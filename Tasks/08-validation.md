# Task 08 — Numerical correctness and integration validation

Status: complete (2026-11-27). Starts with task 03 and gates later work. Delivered a versioned fixture corpus (`tests/fixtures/01..09`) with independently authored analytic and supported-dump fixtures; Rust integration tests (`engine/tests/validation.rs`) for traces, tolerances, KCL residual, determinism, reset, rejected steps, topology edits, invalid parameters and document round trips; Python API tests (`tests/test_api.py`); CI (`.github/workflows/ci.yml`); and a full validation report in `Docs/numerical-validation.md`. Evidence: `cargo test --workspace` 102 passed / 0 failed / 1 ignored; `python tests/smoke.py` SMOKE OK; `python tests/test_api.py` 5 passed; `./tests/run_browser_ui.sh` 13/13. Reference execution was unavailable (source-only donor snapshots), so no donor traces are claimed; donor versions are recorded and trace capture is listed as a follow-up.

Create versioned fixture corpus: hand-calculated DC circuits, RC/RL/RLC transients, diode rectifier, transistor switching, floating node/singular circuits, and representative supported Falstad exports. Record reference snapshot, input, initial conditions, solver settings, sample times and expected values. When reference execution is available, obtain traces from the chosen donor; do not invent golden data. Document build/run steps and any reference limitations.

Compare voltage/current traces using explicit absolute and relative tolerances, aligned simulated times, peak/settling/frequency checks where relevant, and KCL residuals. Explain tolerance selection and intentional differences. Validate convergence, reset, rejected steps, topology edits, invalid parameters and repeated deterministic runs.

Add Rust unit/integration tests, Python API tests and browser smoke tests. Test serialization round trips and unsupported imports. Keep fixtures license-cleared or authored independently; record provenance. Automate regression runs in CI where practical.

Acceptance: report actual pass/fail counts, tolerances and reference versions; no unexplained deviations for supported MVP fixtures. A visually plausible animation alone is not validation.
