# Task 04 — Electrical component models

Status: complete (2026-11-26). Depends on 03. Outputs: \ngine/src/models.rs\ (model registry), \Docs/component-models.md\, new switch/BJT/MOSFET models in \ngine/src/solver.rs\; 28 Rust tests plus the real-worker smoke test pass.

Scope of this slice: wires, ground, resistors, independent voltage/current sources, switches, capacitors, inductors, diodes, simplified BJTs and MOSFETs. Controlled sources, op-amps and digital gates remain deferred; body-diode/bulk, saturation and early effects remain deferred.

Inventory reference model names, serialized IDs, parameters/defaults, terminal ordering, stamps, nonlinear behavior, state updates and scope outputs. Design a Rust model interface for connectivity, stamping, iteration, accepted-step updates, reset and measurement. Keep graphical symbols separate from electrical behavior.

Implement MVP: wire/ground connectivity, resistors, independent voltage/current sources, switches, capacitors, inductors, diodes and basic BJTs/MOSFETs. Preserve polarity and current sign conventions. Next add controlled sources, op-amps, digital gates and other common devices based on demand. Treat idealized devices and reference approximations explicitly; do not claim SPICE equivalence.

For each model record donor path and license approval, equations/assumptions, parameters, initialization and edge cases. Add analytic or reference trace tests; cover reverse bias, switching, zero/invalid values and nonlinear convergence. Separate model metadata for UI editor controls from solver implementation.

Acceptance: model registry, documented supported parameters and terminal mapping, per-model regression tests, unsupported models reported explicitly. Vactrol and other specialized features are deferred to task 10 until core parity.
