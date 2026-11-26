# Task 09 — Performance profiling and optimization

Status: pending. Depends on 08 baseline correctness.

Measure release builds on recorded hardware/OS/toolchain. Compare reference and Electrow using equivalent circuits, simulated duration, step size/settings, accuracy and output sampling. Include small interactive circuits, large passive networks, nonlinear switching circuits and topology edits. Measure simulated seconds per wall second, accepted steps/s, iterations, solve/stamp time, topology rebuild time, allocations/memory, Python bridge overhead, transport bytes and browser frame latency. Report repeated runs and variability.

Profile before optimizing. Investigate matrix structure reuse, sparse/dense crossover, allocations, data locality, batched stepping, bounded scope sampling and frame coalescing. Do not introduce parallelism/GPU or relaxed precision without evidence. Keep optimization changes isolated and rerun task 08 tests for each.

Acceptance: reproducible benchmark harness and baseline report; measured improvements with correctness preserved; bottlenecks and regressions documented. No promised speedup before measurements.
