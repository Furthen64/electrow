# Task 01 — Reference source analysis

Status: pending. Output: docs/reference-analysis.md and a machine-readable source comparison inventory.

Inspect current snapshots in ref_SEVA77/, ref_Vactrol/ and ref_LVLabs/. Record versions, source trees, language, build/runtime dependencies and licensing exceptions. Compare corresponding files using normalized paths, hashes and targeted diffs; distinguish real semantic changes from formatting, headers, generated artifacts and packaging.

Map the actual entry points and call paths for topology, matrix stamping, solve, nonlinear iteration, time stepping, model state, rendering, editing, scope sampling, serialization and desktop integration. Locate component registration and dump identifiers. Trace one resistor/voltage-source circuit and one nonlinear circuit through the solver. Identify source paths and line ranges, not just README claims.

Compare SEVA77 desktop improvements, Vactrol model/audio additions and LVLabs removals. Determine whether LVLabs is leaner in runtime code or only infrastructure. Select a candidate baseline for each subsystem with evidence and alternatives; avoid merging all forks wholesale. Record known differences in numerical behavior and candidate reference fixtures.

Acceptance: all three snapshots accounted for; subsystem map, component/difference matrix, recommended donor per subsystem, licensing follow-ups and baseline selection rationale written. No implementation or reference modifications.
