# Task 03 — Rust circuit simulation core

Status: pending. Depends on 02. Begin numerical tests from task 08 immediately.

Use the selected reference solver to document actual algorithms, node numbering, ground convention, voltage-source unknowns, matrix/RHS stamping, pivoting, convergence checks and time integration before translation. Port behavior in small stages rather than translating the entire UI-driven class hierarchy.

Implement typed circuit graph, component IDs, validated parameters, analysis/topology rebuild, matrix assembly, linear solve and explicit step lifecycle. Use f64 initially. Implement DC resistive circuits and ideal sources first, then transient capacitor/inductor companion models, then nonlinear iteration. Preserve previous accepted state and roll back on rejected steps; document time step selection, iteration limits and failure handling. Avoid silently adding conductance or changing physical assumptions; any stabilization must be explicit and tested.

Start with a straightforward reference-equivalent solver; choose dense/sparse strategy from measured circuit sizes later. Keep allocations and matrix structure visible for profiling. Separate topology changes from per-step value changes. Expose structured diagnostics for floating nodes, singular matrices, invalid values and nonconvergence. No UI or Python dependency inside core solver logic.

Acceptance: Rust unit tests for stamps, matrix solve and state updates; analytical resistor divider, RC/RL transient and nonlinear reference fixtures pass agreed tolerances; deterministic stepping and clear failure diagnostics. Record algorithm/source mapping and any deliberate deviation from reference behavior.
