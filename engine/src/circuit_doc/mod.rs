//! Electrow circuit document: versioned JSON schema (v1).
//!
//! The document is the durable, UI-independent description of a circuit:
//! element ids, model kinds, SI parameters, explicit terminal connectivity
//! (`nodes`), schematic geometry and simulation options. Transient solver
//! state (node voltages, time, iteration counters) is never stored here.

mod parse;
mod serialize;
mod types;
mod validate;

pub use parse::parse_document;
pub use serialize::document_to_json;
pub use types::{
    CircuitDocument, CircuitElement, CircuitOptions, DocDiagnostic, Geometry, CIRCUIT_DOC_FORMAT,
    CIRCUIT_DOC_VERSION, MAX_DOCUMENT_BYTES, MAX_ELEMENTS, MAX_ID_LEN, MAX_TITLE_LEN,
};

use std::collections::HashMap;

/// Convert a validated document into topology operations plus a param table.
/// Params are applied through `set_parameter` after topology assembly.
pub fn document_operations(
    document: &CircuitDocument,
) -> (Vec<crate::solver::Operation>, Vec<(String, HashMap<String, f64>)>) {
    let mut operations = Vec::new();
    let mut param_edits = Vec::new();
    for element in &document.elements {
        operations.push(crate::solver::Operation::Add {
            id: element.id.clone(),
            kind: element.kind.clone(),
            nodes: element.nodes.clone(),
        });
        if !element.params.is_empty() {
            param_edits.push((element.id.clone(), element.params.clone()));
        }
    }
    (operations, param_edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    fn sample_document() -> CircuitDocument {
        parse_document(r#"{
            "format": "electrow-circuit",
            "version": 1,
            "circuit_id": "test-1",
            "title": "divider",
            "options": {"voltage_range": 10.0},
            "elements": [
                {"id": "gnd1", "kind": "ground", "nodes": ["gnd"]},
                {"id": "v1", "kind": "voltage_source", "nodes": ["n1", "gnd"], "params": {"voltage_v": 10.0}},
                {"id": "r1", "kind": "resistor", "nodes": ["n1", "n2"], "params": {"resistance_ohm": 10000.0}}
            ]
        }"#)
        .unwrap()
    }

    #[test]
    fn sample_round_trip() {
        let doc = sample_document();
        let json = document_to_json(&doc);
        let text = json.to_json();
        let reparsed = parse_document(&text).unwrap();
        assert_eq!(doc, reparsed);
    }

    #[test]
    fn export_is_stable() {
        let doc = sample_document();
        let first = document_to_json(&doc).to_json();
        let second = document_to_json(&parse_document(&first).unwrap()).to_json();
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_unsupported_version() {
        let err = parse_document(r#"{"format":"electrow-circuit","version":2,"circuit_id":"x","elements":[]}"#)
            .unwrap_err();
        assert!(err.iter().any(|d| d.code == "unsupported_version"));
    }

    #[test]
    fn rejects_bad_kind_and_params() {
        let err = parse_document(r#"{
            "format": "electrow-circuit", "version": 1, "circuit_id": "x",
            "elements": [
                {"id":"w1","kind":"wire","nodes":["a"]},
                {"id":"r1","kind":"resistor","nodes":["a","b"],"params":{"resistance_ohm":-1.0,"bogus":1.0}},
                {"id":"r1","kind":"resistor","nodes":["a","b"]}
            ]
        }"#)
        .unwrap_err();
        let codes: Vec<_> = err.iter().map(|d| d.code).collect();
        assert!(codes.contains(&"wrong_terminal_count"));
        assert!(codes.contains(&"invalid_param"));
        assert!(codes.contains(&"unknown_param"));
        assert!(codes.contains(&"duplicate_id"));
    }

    #[test]
    fn accepts_missing_optional_fields() {
        let doc = parse_document(r#"{"format":"electrow-circuit","version":1,"circuit_id":"x","elements":[]}"#).unwrap();
        assert!(doc.title.is_none());
        assert_eq!(doc.options.max_time_step_s, 5e-6);
    }

    #[test]
    fn rejects_oversized_document() {
        let big = format!("{{\"format\":\"electrow-circuit\",\"version\":1,\"circuit_id\":\"x\",\"elements\":[],\"padding\":\"{}\"}}", "a".repeat(MAX_DOCUMENT_BYTES));
        let err = parse_document(&big).unwrap_err();
        assert_eq!(err[0].code, "document_too_large");
    }

    #[test]
    fn parse_is_diagnostic_not_panicky() {
        let err = parse_document("not json").unwrap_err();
        assert_eq!(err[0].code, "invalid_json");
    }

    #[test]
    fn operations_apply_params() {
        let doc = sample_document();
        let (ops, edits) = document_operations(&doc);
        assert_eq!(ops.len(), 3);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn json_does_not_panic_on_weird_values() {
        let _ = parse("{}").unwrap();
    }
}
