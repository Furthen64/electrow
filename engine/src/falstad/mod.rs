//! Falstad text-dump import.
//!
//! Format (from reference snapshots, see Docs/reference-analysis.md §5.1/§6):
//!   - One `$ flags maxTimeStep speed currentBar voltageRange [powerBar [minTimeStep]]` header line.
//!   - One element per line: `<type> <x1> <y1> <x2> <y2> <flags> [params...]`.
//!   - Type ids are either a single character (`r`, `v`, ...) or a numeric int.
//!
//! Supported elements map onto the Electrow component set (wire, ground,
//! resistor, voltage/current sources, capacitor, inductor, diode, switch,
//! BJT, MOSFET). Geometry is converted to connectivity by exact endpoint
//! coordinate matching, which is the same rule the reference renderer uses
//! for two-terminal elements (posts are the two endpoints). Unsupported or
//! malformed lines are collected as `UnsupportedRecord` diagnostics; they
//! never silently substitute a different model. Three-terminal elements
//! (BJT/MOSFET) are reported as unsupported: the text dump only stores two
//! endpoints, and reproducing Falstad's post geometry requires a geometry
//! port that is out of scope here.

mod elements;
mod geometry;
mod parser;

use parser::{split_tokens, RawElement, MAX_ELEMENTS, MAX_LINE_LEN, MAX_TEXT_BYTES};
use crate::circuit_doc::{
    CircuitDocument, CircuitOptions, DocDiagnostic,
};

#[derive(Clone, Debug, PartialEq)]
pub struct UnsupportedRecord {
    pub line: usize,
    pub type_token: String,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct FalstadImport {
    pub document: CircuitDocument,
    pub unsupported: Vec<UnsupportedRecord>,
    /// Warnings about supported elements whose parameters were partially
    /// ignored (transient state the model does not store).
    pub warnings: Vec<String>,
}

/// Parse a Falstad text dump into a circuit document.
///
/// Unsupported records are collected, not fatal; the document carries the
/// supported elements and their explicit nodes. Fatal errors (oversized
/// input, unparseable options header, structurally broken lines) are
/// rejected through the returned error.
pub fn parse_falstad_text(text: &str) -> Result<FalstadImport, DocDiagnostic> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(DocDiagnostic::err(
            "document_too_large",
            format!("text dump is {} bytes, limit is {}", text.len(), MAX_TEXT_BYTES),
        ));
    }

    let mut options = CircuitOptions::default();
    let mut saw_options = false;
    let mut raw_elements: Vec<RawElement> = Vec::new();
    let mut unsupported: Vec<UnsupportedRecord> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for (index, raw_line) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('$') {
            if saw_options {
                warnings.push(format!("line {line_no}: duplicate options line ignored"));
                continue;
            }
            saw_options = true;
            options = parser::parse_options_line(&split_tokens(line), line_no)?;
            continue;
        }
        if line.len() > MAX_LINE_LEN {
            return Err(DocDiagnostic::err(
                "line_too_long",
                format!("line {line_no}: exceeds {MAX_LINE_LEN} chars"),
            ));
        }
        match parser::parse_element_line(line_no, line) {
            Ok(Some(raw)) => raw_elements.push(raw),
            Ok(None) => {}
            Err(diagnostic) => {
                if diagnostic.code == "unsupported_record" {
                    let token = split_tokens(line).first().cloned().unwrap_or_default();
                    unsupported.push(UnsupportedRecord {
                        line: line_no,
                        type_token: token,
                        reason: diagnostic.message,
                    });
                } else {
                    return Err(diagnostic);
                }
            }
        }
    }

    if raw_elements.len() > MAX_ELEMENTS {
        return Err(DocDiagnostic::err(
            "too_many_elements",
            format!("text dump has {} elements, limit is {MAX_ELEMENTS}", raw_elements.len()),
        ));
    }

    let mut elements = Vec::new();
    for raw in &raw_elements {
        let kind = elements::map_kind(raw).unwrap_or("unknown");
        if kind == "bjt" || kind == "mosfet" {
            unsupported.push(elements::make_unsupported(
                raw,
                "3-terminal geometry cannot be recovered from the text dump",
            ));
            continue;
        }
        match elements::parse_element(raw, &mut warnings) {
            Ok(element) => elements.push(element),
            Err(record) => unsupported.push(record),
        }
    }

    let elements = geometry::assign_nodes(elements);

    let document = CircuitDocument {
        circuit_id: String::new(), // filled in by caller
        title: None,
        options,
        elements,
    };

    Ok(FalstadImport {
        document,
        unsupported,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIVIDER: &str = "\
$ 1 5.0E-6 10 63 10.0 62
v 112 368 112 48 0 0 40.0 10.0 0.0
w 112 48 240 48 0
r 240 48 240 208 0 10000
r 240 208 240 368 0 10000
w 112 368 240 368 0
";

    #[test]
    fn parses_options_line() {
        let import = parse_falstad_text(DIVIDER).unwrap();
        assert_eq!(import.document.options.max_time_step_s, 5.0e-6);
        assert_eq!(import.document.options.voltage_range, 10.0);
        assert_eq!(import.document.options.flags, 1);
    }

    #[test]
    fn parses_elements_and_merges_nodes() {
        let import = parse_falstad_text(DIVIDER).unwrap();
        let doc = &import.document;
        assert_eq!(doc.elements.len(), 5); // v + w + r + r + w
        let source = doc.elements.iter().find(|e| e.kind == "voltage_source").unwrap();
        assert_eq!(source.params.get("voltage_v"), Some(&10.0));
        let resistors: Vec<_> = doc.elements.iter().filter(|e| e.kind == "resistor").collect();
        assert_eq!(resistors.len(), 2);
        assert_eq!(resistors[0].params.get("resistance_ohm"), Some(&10000.0));
        // The first resistor's first node must equal the first wire's second node (240,48).
        let first_wire = doc.elements.iter().find(|e| e.kind == "wire").unwrap();
        assert_eq!(resistors[0].nodes[0], first_wire.nodes[1]);
        // The first resistor's second node must equal the second resistor's first node.
        assert_eq!(resistors[0].nodes[1], resistors[1].nodes[0]);
    }

    #[test]
    fn reports_unsupported_scope_and_numeric_ids() {
        let text = "\
$ 1 5.0E-6 10 63 10.0
r 100 100 100 200 0 1000
403 300 300 340 300 1
";
        let import = parse_falstad_text(text).unwrap();
        assert_eq!(import.document.elements.len(), 1);
        assert_eq!(import.unsupported.len(), 1);
        assert_eq!(import.unsupported[0].type_token, "403");
    }

    #[test]
    fn rejects_bad_options() {
        let err = parse_falstad_text("$ bogus 5.0E-6 10 63 10.0\nr 0 0 1 1 0 1000\n").unwrap_err();
        assert_eq!(err.code, "invalid_options_line");
    }

    #[test]
    fn switch_position_mapping() {
        let text = "\
$ 0 5.0E-6 10 0 5.0
s 100 100 100 200 0 true true
";
        let import = parse_falstad_text(text).unwrap();
        let element = import.document.elements.iter().find(|e| e.kind == "switch").unwrap();
        assert_eq!(element.params.get("closed"), Some(&0.0));
    }

    #[test]
    fn bjt_and_mosfet_are_reported_unsupported() {
        let text = "\
$ 0 5.0E-6 10 0 5.0
t 100 100 100 200 0 1 0.5 0.5 100 default
f 100 100 100 200 1 1.5 0.02
";
        let import = parse_falstad_text(text).unwrap();
        assert_eq!(import.document.elements.len(), 0);
        assert_eq!(import.unsupported.len(), 2);
        assert!(import.unsupported.iter().all(|u| u.reason.contains("3-terminal")));
    }

    #[test]
    fn current_source_voltage_limit_reported() {
        let text = "\
$ 0 5.0E-6 10 0 5.0
i 100 100 100 200 0 0.001 5.0
";
        let import = parse_falstad_text(text).unwrap();
        assert_eq!(import.document.elements.len(), 0);
        assert_eq!(import.unsupported.len(), 1);
        assert!(import.unsupported[0].reason.contains("voltage-limited"));
    }

    #[test]
    fn inductor_imports_inductance_and_warns() {
        let text = "\
$ 0 5.0E-6 10 0 5.0
l 100 100 100 200 0 1.0e-3 0.0 0.0 0.0
";
        let import = parse_falstad_text(text).unwrap();
        assert_eq!(import.document.elements.len(), 1);
        assert_eq!(import.document.elements[0].params.get("inductance_h"), Some(&0.001));
        assert!(!import.warnings.is_empty());
    }

    #[test]
    fn rejects_oversized_text() {
        let text = format!("r 0 0 1 1 0 1000\n{}", "x".repeat(MAX_TEXT_BYTES));
        assert!(parse_falstad_text(&text).is_err());
    }

    #[test]
    fn rejects_structurally_broken_element_line() {
        let text = "r not_a_number 0 1 1 0 1000\n";
        let err = parse_falstad_text(text).unwrap_err();
        assert_eq!(err.code, "malformed_line");
    }
}
