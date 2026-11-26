# Circuit format and Falstad import (Task 05)

Status: complete (2026-11-26). Delivered: `engine/src/circuit_doc/` (versioned Electrow circuit JSON, v1, with validation and stable serialization; module layout `types`/`parse`/`validate`/`serialize`), `engine/src/falstad/` (Falstad text import: `parser`/`elements`/`geometry`; three-terminal geometry ported exactly), `engine/src/xml_import.rs` (reference-format XML import, same model semantics), `engine/src/import_common.rs` (shared post-geometry/node-merging pipeline), `load_circuit` with `format: "electrow-json"` / `"falstad-text"` / `"text"` / `"xml"`, `export_circuit`, and a 70-test suite.

## Electrow circuit JSON (schema v1)

| Field | Type | Required | Notes |
|---|---|---|---|
| `format` | string | yes | must be `"electrow-circuit"` |
| `version` | number | yes | must be `1` |
| `circuit_id` | string | yes | 1–128 chars |
| `title` | string | no | ≤ 1024 chars |
| `options` | object | no | see below |
| `elements` | array | yes | ≤ 4096 entries |

`options`: `flags` (u32, informational), `max_time_step_s` (0, 1], `min_time_step_s` (0, max], `adjust_time_step` (bool), `auto_dc_on_reset` (bool), `voltage_range` (0, 1e6]. Defaults: 5e-6 / 50e-12 / false / false / 5.0.

`elements[]` item:

| Field | Type | Required | Notes |
|---|---|---|---|
| `id` | string | yes | unique, 1–128 chars |
| `kind` | string | yes | must be a registered model kind |
| `nodes` | array of string | yes | length = terminal count, every node ≤ 128 chars |
| `params` | object | no | keys and values validated against the model registry |
| `geometry` | object | no | `x1,y1,x2,y2,flags` (display only, not used to derive nodes) |

Validation (via `circuit_doc::parse_document`) rejects: oversized input (1 MiB), JSON parse errors, wrong/missing `format` or `version`, missing/duplicate/invalid element ids, unknown kinds, wrong terminal counts, unknown or out-of-range parameters (binary/positive/finite), out-of-range geometry, and malformed options. Diagnostics carry a `code`, `message` and optional `element_id`.

Round-trip: `load_circuit` → engine → `export_circuit` yields the same document (same element order, sorted params, stable key order). Verified by `document_tests::electrow_json_round_trip_and_loads` and `export_is_stable`.

## Geometry → connectivity

The Electrow document is connectivity-explicit: `nodes` are assigned by the producer and validated, not derived. For Falstad text import, connectivity is derived by exact endpoint-coordinate matching — the same rule the reference renderer uses for two-terminal elements, whose posts are exactly their two endpoints (`CircuitElm.getPost`, Vactrol `:817-819`; `calcLeads` only affects drawing stubs). All ground elements merge into one `gnd` reference node.

Three-terminal elements (BJT/MOSFET) are supported: the reference's post-geometry is ported exactly (`import_common::bjt_posts`/`mosfet_posts` reproduce `TransistorElm::setPoints` / `MosfetElm::setPoints` integer math), so a wire endpoint can merge with a transistor's computed terminal post by exact coordinate matching. Geometry is geometry-driven only — it is used at import time to produce explicit `nodes`; the stored document itself is connectivity-explicit like any other. Records that cannot map faithfully (4-terminal MOSFETs with a bulk terminal, MOSFET body-diode models, custom transistor models, zero-length elements) are collected as `unsupported` records; they are never silently substituted.

## Falstad text dump format (from reference snapshots)

Oracle: `ref_Vactrol/src/com/lushprojects/circuitjs1/client/CircuitLoader.java` (`readCircuit(byte[])`, `readOptions`, `readCircuitFlags`) and the legacy `CirSim.java` equivalents. See Docs/reference-analysis.md §5.1.

- Options header (optional, only if first line starts with `$`):
  `$ <flags> <maxTimeStep> <speed> <currentBar> <voltageRange> [powerBar [minTimeStep]]`
  - flags: bit 0 dots, bit 1 smallGrid, bit 2 hide volts (inverse), bit 3 power on, bit 4 hide values (inverse), bit 6 adjustTimeStep, bit 7 autoDCOnReset. We import flags intact plus derive `adjust_time_step`/`auto_dc_on_reset`; speed and currentBar are UI scales and are not stored.
- Element lines: `<type> <x1> <y1> <x2> <y2> <flags> [params...]`, one per line. Type is a single character or numeric id; numeric ids are not supported (scopes = 403, models = 32/34/38, etc.) and are reported.
- Special lines: `!`, `%`, `?`, `B` are ignored (afilter bookkeeping); `h` (hint), `o` (scope) and `.` (custom composite) are reported unsupported.

Supported single-character ids in this slice and the parameter mapping:

| id | Kind | Params (SI) | Notes |
|---|---|---|---|
| `w` | wire | — | |
| `g` | ground | — | optional cosmetic symbolType token ignored |
| `r` | resistor | `resistance_ohm` | |
| `v` | voltage_source | `voltage_v = maxVoltage + bias` | only DC (waveform 0); legacy 4-token form supported |
| `i` | current_source | `current_a` | voltage-limited (maxVoltage > 0) reported unsupported; 0 → 0.01 (reference behavior) |
| `c` | capacitor | `capacitance_f` | initial state/series resistance reported as warnings (not stored) |
| `l` | inductor | `inductance_h` | initial state/saturation reported as warnings |
| `d` | diode | none (default model) | custom model names reported unsupported; fwdrop reported as warning |
| `s` | switch | `closed` (1/0) | legacy position tokens `true`/`false` supported; momentary/label reported as warning |
| `t` | bjt | `pnp` (0/1), `beta` | canonical token form `[pnp, vbe, lastvbc, beta, modelName?]`; legacy short forms supported; initial state reported as warning |
| `f` | mosfet | `pnp`, `vt`, `beta` | legacy `[vt, beta]` form; legacy flags 32/64/128 reported/warned; custom models unsupported |

## Commands

- `load_circuit`: `{format: "electrow-json" | "falstad-text" | "text" | "xml", dump: string}`. For all formats this parses, validates, replaces the active circuit and reports unsupported records/warnings in `result`. The two file formats are interchangeable aliases: `"text"` behaves exactly like `"falstad-text"` and `"xml"` parses the reference-format XML document.
- `export_circuit`: returns the loaded document as JSON (`circuit_id`, `title`, `options`, `elements` with sorted params). Fails with `no_document` if nothing has been loaded.

## Limitations / follow-ups

- Reference-format XML import (`<circuit>` root; elements `w g r v i c l d s t f` with attributes matching the reference's `XMLSerializer.dumpAttr`; options `f ts ic cb vr pb mts st`) — supported. Scope records, hint lines, models, adjustables, subcircuits, tests, test scopes, custom logic/composite and unknown tags are reported unsupported.
- Reference comparison: text and XML imports of the same circuit produce identical documents (verified by `xml_matches_text_import_for_divider`).
- Scope records, controlled sources, op-amps and digital gates remain unsupported and are reported, not substituted.
- Initial transient state (capacitor voltages, inductor currents, BJT vbe/vbc) is not imported and is reported as a warning; the imported circuit starts from zero state.

## Acceptance checklist

- [x] Round-trip Electrow JSON (load → export → re-parse, byte-stable)
- [x] Known Falstad fixtures imported with correct topology and model parameters (verified on a divider and on BJT/MOSFET fixtures with computed post geometry)
- [x] Unsupported records reported (scopes, numeric ids, 3-terminal kind, custom models)
- [x] Malformed input rejected safely (oversized/malformed/structurally broken lines)
- [x] Reference comparison: options header flags/semantics follow the Vactrol/LVLabs/SEVA77 `CircuitLoader`/`dumpCircuit` behavior
- [x] Reference comparison suite: text and XML imports of the same circuit produce identical documents and rendered topology (divider + BJT/MOSFET fixtures)
