# Code provenance

## Import gate

Before copying or porting code from a candidate repository into Electrow, verify the applicable license of the selected material, compatibility with the intended distribution license, and any required attribution or dependency obligations. Investigate Git history selectively where current licensing evidence is ambiguous. Do not import unresolved or incompatible material until the issue is resolved.

Preserve upstream copyright and license notices and identify modifications as required. Independently implemented functionality should be distinguished from copied or ported code.

## Import register

One behavior-derived implementation has been recorded (Task 10). No donor source text was copied.

| Feature | Upstream repository | Commit | Source path | License | Integration method | Destination | Verification evidence |
|---|---|---|---|---|---|---|---|
| Vactrol optocoupler behavior | ref_Vactrol (Vactrol fork) | checked-out snapshot 2026-11-26 | `src/com/lushprojects/circuitjs1/client/VactrolModel.java:19-51`, `VactrolElm.java:31-34,86-98,285-309` | GPL-3.0-or-later (SPDX headers, Copyright 2026 Nerding and Hoarding) | Independent Rust implementation of documented transfer/default equations; no source text copied; compatible with Electrow GPL-3.0-or-later policy | `engine/src/models.rs`, `engine/src/solver.rs`, `tests/fixtures/10-vactrol.json` | `vactrol_transfer_matches_documented_checkpoints`, `vactrol_conductance_update_is_exact_and_bounded`, `vactrol_fixture`; `cargo test --workspace` |
| Browser New/Open/Save workflow pattern | ref_SEVA77 | checked-out snapshot 2026-11-26 | `patches/Toolbar.patch:35-44`, `patches/CirSim.patch` | GPL-2.0-or-later (interaction pattern only, no code copied) | Independent vanilla-JS reimplementation against Electrow's own endpoints | `web/`, `tests/browser_ui.cjs` | browser harness file round-trip and keyboard-shortcut checks |

See [REFERENCES.md](REFERENCES.md) for candidate repositories and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for notices.
