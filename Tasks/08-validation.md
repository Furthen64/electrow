# Task 08 — Numerical correctness and integration validation

Status: pending. Starts with task 03 and gates later work.

Create versioned fixture corpus: hand-calculated DC circuits, RC/RL/RLC transients, diode rectifier, transistor switching, floating node/singular circuits, and representative supported Falstad exports. Record reference snapshot, input, initial conditions, solver settings, sample times and expected values. When reference execution is available, obtain traces from the chosen donor; do not invent golden data. Document build/run steps and any reference limitations.

Compare voltage/current traces using explicit absolute and relative tolerances, aligned simulated times, peak/settling/frequency checks where relevant, and KCL residuals. Explain tolerance selection and intentional differences. Validate convergence, reset, rejected steps, topology edits, invalid parameters and repeated deterministic runs.

Add Rust unit/integration tests, Python API tests and browser smoke tests. Test serialization round trips and unsupported imports. Keep fixtures license-cleared or authored independently; record provenance. Automate regression runs in CI where practical.

Acceptance: report actual pass/fail counts, tolerances and reference versions; no unexplained deviations for supported MVP fixtures. A visually plausible animation alone is not validation.
