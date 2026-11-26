# Task 05 — Circuit document and compatibility

Status: partial (2026-11-26). Depends on 02 and 04. Done: versioned Electrow circuit JSON v1 (`engine/src/circuit_doc.rs`) with validation/serialization; Falstad text import for two-terminal components (`engine/src/falstad.rs`); `load_circuit` extended with `electrow-json`/`falstad-text` formats; `export_circuit`; 52 Rust tests + smoke test all pass; details in Docs/circuit-format.md. Remaining: BJT/MOSFET text import (3-terminal geometry), XML import, reference comparison suite.

Define versioned Electrow circuit JSON containing stable component IDs, model identifiers, validated SI parameters, terminal connectivity, schematic geometry, wire endpoints, labels and scope configuration. Persist document state separately from transient solver state. Keep Rust authoritative for electrical connectivity; define how geometric connections and explicit junctions are converted to nodes.

Inspect each reference import/export parser and dump format, including options, component IDs, flags, coordinates, parameter defaults and scope records. Implement Falstad text import for supported components; retain or explicitly report unknown records and unsupported parameters. Do not silently substitute a resistor or drop electrically significant records. Define export compatibility limits and preserve original input where useful.

Validate untrusted file sizes, numeric ranges, node references and schema versions. Provide clear diagnostics with record/line context. Keep save/load deterministic; separate UI layout data from electrical state and define reset behavior.

Acceptance: round-trip Electrow JSON, known Falstad fixtures imported with correct topology and model parameters, unsupported records reported, malformed input rejected safely, reference comparison tests for imported circuits.
