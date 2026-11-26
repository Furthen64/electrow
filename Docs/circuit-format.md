# Circuit format and Falstad import (Task 05)

Status: partial (2026-11-26). Delivered: `engine/src/circuit_doc.rs` (versioned Electrow circuit JSON, v1, with validation and stable serialization), `engine/src/falstad.rs` (Falstad text import), `load_circuit` with `format: "electrow-json"` / `"falstad-text"` (and preserved legacy `"text"`/`"xml"`), `export_circuit`, and a 16-test suite. Not complete: BJT/MOSFET import (documented limitation below).

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

Three-terminal elements (BJT/MOSFET) are **not** imported in this slice: the text format stores only two endpoints, and reproducing Falstad's post geometry (`interpPoint2`, `dsign`, `pnp` flag) requires a geometry port that is out of scope. These lines are collected as `unsupported` records with reason `3-terminal geometry cannot be recovered from the text dump`; they are never silently substituted.

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
| `t` | bjt | *(unsupported — geometry)* | see above |
| `f` | mosfet | *(unsupported — geometry)* | see above |

## Commands

- `load_circuit`: `{format: "electrow-json" | "falstad-text" | "text" | "xml", dump: string}`. For the two supported formats this parses, validates, replaces the active circuit and reports unsupported records/warnings in `result`. For legacy `text`/`xml` the previous behavior is preserved.
- `export_circuit`: returns the loaded document as JSON (`circuit_id`, `title`, `options`, `elements` with sorted params). Fails with `no_document` if nothing has been loaded.

## Limitations / follow-ups

- BJT/MOSFET text import (3-terminal geometry) is deferred; see above. This keeps the acceptance criteria honest rather than claiming parity we have not verified.
- XML import is not implemented (text is the chosen interop path; XML is a follow-up).
- Scope records, controlled sources, op-amps and digital gates remain unsupported and are reported, not substituted.
- Initial transient state (capacitor voltages, inductor currents, BJT vbe/vbc) is not imported and is reported as a warning; the imported circuit starts from zero state.

## Acceptance checklist

- [x] Round-trip Electrow JSON (load → export → re-parse, byte-stable)
- [x] Known Falstad fixtures imported with correct topology and model parameters (verified on voltdivide-style divider; BJT/MOSFET deferred)
- [x] Unsupported records reported (scopes, numeric ids, 3-terminal kind, custom models)
- [x] Malformed input rejected safely (oversized/malformed/structurally broken lines)
- [x] Reference comparison: options header flags/semantics follow the Vactrol/LVLabs/SEVA77 `CircuitLoader`/`dumpCircuit` behavior
- [ ] Reference comparison suite for the imported circuits (blocked on 3-terminal import and on fixture corpus)
