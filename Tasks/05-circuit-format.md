# Task 05 — Circuit document and compatibility

Status: complete (2026-11-26). Depends on 02 and 04.

Define versioned Electrow circuit JSON containing stable component IDs, model identifiers, validated SI parameters, terminal connectivity, schematic geometry, wire endpoints, labels and scope configuration. Persist document state separately from transient solver state. Keep Rust authoritative for electrical connectivity; define how geometric connections and explicit junctions are converted to nodes.

Inspect each reference import/export parser and dump format, including options, component IDs, flags, coordinates, parameter defaults and scope records. Implement Falstad text import for supported components; retain or explicitly report unknown records and unsupported parameters. Do not silently substitute a resistor or drop electrically significant records. Define export compatibility limits and preserve original input where useful.

Validate untrusted file sizes, numeric ranges, node references and schema versions. Provide clear diagnostics with record/line context. Keep save/load deterministic; separate UI layout data from electrical state and define reset behavior.

Acceptance: round-trip Electrow JSON, known Falstad fixtures imported with correct topology and model parameters, unsupported records reported, malformed input rejected safely, reference comparison tests for imported circuits.

## Delivered

- `engine/src/circuit_doc/` (`types`/`parse`/`validate`/`serialize`/`mod`): Electrow circuit JSON v1 — stable ids, registered model kinds, validated SI parameters, explicit connectivity, geometry kept display-only, options (flags/max/min timestep/voltage range/power and auto-reset flags). Diagnostics carry codes, messages and element context; 1 MiB input cap; 4096 element cap.
- `engine/src/falstad/` (`parser`/`elements`/`geometry`): Falstad text-dump import for two-terminal components plus BJT/MOSFET (post geometry ported exactly from `TransistorElm::setPoints`/`MosfetElm::setPoints` integer math). Wire endpoints merge with computed transistor posts by exact coordinate matching, same rule as the reference renderer (`CircuitElm.getPost`).
- `engine/src/xml_import.rs`: reference-format XML import (`<circuit>` root; elements w/g/r/v/i/c/l/d/s/t/f; options f/ts/ic/cb/vr/pb/mts/st). Same model semantics as the text importer; custom-model records and scope/hint/model tags are reported unsupported.
- `engine/src/import_common.rs`: shared post-geometry and coordinate-merge pipeline used by both importers.
- `load_circuit` with `format: "electrow-json"` / `"falstad-text"` / `"text"` / `"xml"` (`text`/`xml` are aliases; both file formats are real implementations, not stubs); `export_circuit` returns the stable JSON document.
- Docs/circuit-format.md: schema, mapping tables, geometry rules, diagnostics, acceptance checklist.

## Explicitly deferred (and reported, not substituted)

- Three-terminal geometry with a bulk terminal (MOSFET body terminal) and MOSFET body-diode models are rejected as unsupported.
- Custom BJT/MOSFET/diode model names are reported unsupported (default model only).
- Initial transient state (capacitor voltages, inductor currents, BJT vbe/vbc) is not imported; reported as warnings and the circuit starts from zero state.
- Scope records, hints, digital logic, models, adjustables, subcircuits, compose/test data and unknown tags are reported unsupported.
- Parameters that the Electrow models do not store (diode fwdrop, voltage-source internal resistance, switch momentary/label/key, frequency/duty cycle, MOSFET bulk display/digital display flags) are imported as warnings, not silent drops.

## Tests

70 unit tests across `circuit_doc` (validation/round-trip/stability), `falstad` (options, topology, node merging, unsupported records, BJT post-geometry merging, MOSFET legacy flags), `xml_import` (options, per-kind parameters, bjt/mosfet modern+legacy forms, malformed input, comparison fixture), `import_common` (shared geometry) and `lib.rs` (engine load/export through the public command surface).
