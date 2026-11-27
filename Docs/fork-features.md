# Fork feature selection (Task 10)

Decisions are based on the actual donor source inspected in Task 01, not on README claims. The import gate in [PROVENANCE.md](../PROVENANCE.md) applies to every adopted feature.

| Candidate | Donor evidence | Decision | Rationale / ownership |
|---|---|---|---|
| Vactrol optocoupler (dump id 391) | `ref_Vactrol/.../VactrolModel.java:19-51` (GPL-3.0-or-later, 2026 Nerding and Hoarding); `VactrolElm.java:1-6,31-34,86-98,285-309` (same license) | **Adopt** | Distinctive, self-contained physics with an exact first-order conductance update and documented VTL5C3-like defaults. Rust engine owns state; independent Rust implementation of the documented equations, not copied source. Tests in `engine/tests/validation.rs`; no frontend solver logic. |
| Keyboard/file workflow (New/Open/Save/Save As, unsaved state) | `ref_SEVA77/patches/Toolbar.patch:35-44`, `patches/CirSim.patch` (GPL-2.0-or-later); browser already has file endpoints from Task 06 | **Adopt (browser-only)** | Re-implement the interaction pattern in vanilla JS against our existing endpoints; no donor code copied. Keyboard shortcuts and unsaved-change badge in `web/`, verified by the browser harness. |
| BJT avalanche / zener + transistor noise | `BjtAvalancheModel.java:19-112`, `TransistorModel.java:150-160,206-262` (GPL-3.0-or-later) | **Defer** | Stateful hysteretic branches and synthetic noise need a documented seeding policy and separate fixture corpus before touching the nonlinear solver; higher regression risk than the optocoupler. |
| Audio export | `AudioOutputElm.java:23-24,163-247,279-365` (GPL-3.0-or-later) | **Defer** | The donor forces a global timestep of `1/sampleRate`; our engine owns adaptive timestep and simulated/wall separation. Needs a deliberate resampling/clipping design and WAV tests before implementation. |
| Transistor presets | `TransistorModel.java:158-160,206-262` (GPL-3.0-or-later) | **Adopt as metadata only** | Preset constants are UI/editor metadata; adding them to the parameter panel does not change solver behavior. Documented in `Docs/component-models.md`. |
| LVLabs theming / infrastructure simplifications | `LVLabs/Theme.java:25-156`, reference-analysis §7 (GPL-2.0-or-later with GPLv3 header ambiguity in `build.gradle`) | **Reject** | Our CSS already uses variables, and the rest is repository/build simplification with no runtime value. The GPL header ambiguity makes reuse unattractive. |
| Contributor-only code (LDRElm and similar exceptions) | reference-analysis §10 | **Reject** | License/provenance unresolved; do not import until verified or independently replaced. |

## Adopted subtasks

1. **Vactrol model** — donor files, equations and defaults above; destination `engine/src/models.rs`, `engine/src/solver.rs`, `engine/tests/validation.rs`; provenance recorded in `PROVENANCE.md` and `THIRD_PARTY_NOTICES.md`.
2. **Browser workflow shortcuts** — donor interaction pattern above; destination `web/js/editor.js`, `web/app.js`, `tests/browser_ui.cjs`; no donor code copied.
3. **Transistor presets** — donor constants above; destination `web/index.html`, `web/app.js`; metadata only, no numerical changes.

## Implementation status (2026-11-27)

- **Vactrol model — adopted and implemented.** `models.rs` registers a four-terminal `vactrol` with the documented defaults; `solver.rs` stamps the LED companion and isolated cell conductance, advances the cell with the exact first-order update and commits state without overwriting the new conductance. Fixture `tests/fixtures/10-vactrol.json`, unit tests for transfer checkpoints and the update rule, and a document round-trip test all pass. The Falstad/XML importers still report dump id `391` as unsupported: the donor's four-post geometry is not ported.
- **Browser workflow — adopted and implemented.** `Ctrl+S` saves, `Ctrl+O` loads, `Ctrl+N` creates a new circuit; an unsaved-changes badge tracks edits and clears on save/load/new. The browser harness checks all three shortcuts and the badge (15/15 checks).
- **BJT presets — adopted as metadata.** The parameter panel maps 2N2222/2N2222A to `beta=200, is_a=14.34e-15` and 2N3904 to `beta=200, is_a=6.734e-15`; alpha, Early effect, junction capacitance and avalanche parameters are deliberately omitted because the simplified BJT model does not use them. Verified by the browser harness.

Deliberate deviations: reverse breakdown of the input LED, avalanche/noise and audio capture are not part of the adopted Vactrol model; browser file operations stay loopback-only with the existing path guard.
