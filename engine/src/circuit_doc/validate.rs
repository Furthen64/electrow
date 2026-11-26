//! Schema validation (format, version, ids, options, elements, params).

use super::types::{DocDiagnostic, MAX_ELEMENTS, MAX_ID_LEN, MAX_TITLE_LEN};
use super::CIRCUIT_DOC_FORMAT;
use crate::json::Value;
use crate::models;
use std::collections::HashSet;

pub fn parse_document_value(value: &Value, diagnostics: &mut Vec<DocDiagnostic>) {
    if !matches!(value, Value::Object(_)) {
        diagnostics.push(DocDiagnostic::err("invalid_document", "document must be a JSON object"));
        return;
    }

    let format = value.get("format").and_then(Value::as_str);
    if format != Some(CIRCUIT_DOC_FORMAT) {
        diagnostics.push(DocDiagnostic::err(
            "invalid_format",
            format!("format must be {CIRCUIT_DOC_FORMAT:?}"),
        ));
    }

    let version = value.get("version").and_then(Value::as_u64);
    match version {
        Some(1) => {}
        Some(other) => diagnostics.push(DocDiagnostic::err(
            "unsupported_version",
            format!("unsupported document version {other} (expected 1)"),
        )),
        None => diagnostics.push(DocDiagnostic::err("missing_version", "version must be a number")),
    }

    match value.get("circuit_id").and_then(Value::as_str) {
        Some(id) if !id.is_empty() && id.len() <= MAX_ID_LEN => {}
        Some(_) => diagnostics.push(DocDiagnostic::err(
            "invalid_circuit_id",
            "circuit_id must be a non-empty string of at most 128 chars",
        )),
        None => diagnostics.push(DocDiagnostic::err(
            "invalid_circuit_id",
            "circuit_id must be a non-empty string",
        )),
    }

    if let Some(title) = value.get("title") {
        match title.as_str() {
            Some(t) if t.len() <= MAX_TITLE_LEN => {}
            Some(_) => diagnostics.push(DocDiagnostic::err(
                "invalid_title",
                format!("title must be at most {MAX_TITLE_LEN} chars"),
            )),
            None => diagnostics.push(DocDiagnostic::err("invalid_title", "title must be a string")),
        }
    }

    validate_options(value.get("options"), diagnostics);

    let elements = value.get("elements").and_then(Value::as_array);
    match elements {
        Some(values) => {
            if values.len() > MAX_ELEMENTS {
                diagnostics.push(DocDiagnostic::err(
                    "too_many_elements",
                    format!("document has {} elements, limit is {MAX_ELEMENTS}", values.len()),
                ));
            }
            let mut seen = HashSet::new();
            for (index, element) in values.iter().enumerate() {
                validate_element(element, index, &mut seen, diagnostics);
            }
        }
        None => diagnostics.push(DocDiagnostic::err(
            "missing_elements",
            "elements must be an array",
        )),
    }
}

fn validate_options(value: Option<&Value>, diagnostics: &mut Vec<DocDiagnostic>) {
    let Some(value) = value else { return };
    if !matches!(value, Value::Object(_)) {
        diagnostics.push(DocDiagnostic::err("invalid_options", "options must be an object"));
        return;
    }
    let max = value.get("max_time_step_s").and_then(Value::as_f64).unwrap_or(5e-6);
    let min = value.get("min_time_step_s").and_then(Value::as_f64).unwrap_or(50e-12);
    if !max.is_finite() || max <= 0.0 || max > 1.0 {
        diagnostics.push(DocDiagnostic::err(
            "invalid_options",
            "options.max_time_step_s must be in (0, 1]",
        ));
    }
    if !min.is_finite() || min <= 0.0 || min > max {
        diagnostics.push(DocDiagnostic::err(
            "invalid_options",
            "options.min_time_step_s must be in (0, max_time_step_s]",
        ));
    }
    if let Some(range) = value.get("voltage_range").and_then(Value::as_f64) {
        if !range.is_finite() || range <= 0.0 || range > 1e6 {
            diagnostics.push(DocDiagnostic::err(
                "invalid_options",
                "options.voltage_range must be in (0, 1e6]",
            ));
        }
    }
}

fn validate_element(
    element: &Value,
    index: usize,
    seen: &mut HashSet<String>,
    diagnostics: &mut Vec<DocDiagnostic>,
) {
    let Some(id) = element.get("id").and_then(Value::as_str) else {
        diagnostics.push(DocDiagnostic::err(
            "missing_id",
            format!("element #{index}: id must be a non-empty string"),
        ));
        return;
    };
    if id.is_empty() || id.len() > MAX_ID_LEN {
        diagnostics.push(DocDiagnostic::for_element(
            "invalid_id",
            "id must be a non-empty string of at most 128 chars",
            id,
        ));
        return;
    }
    if !seen.insert(id.to_string()) {
        diagnostics.push(DocDiagnostic::for_element(
            "duplicate_id",
            format!("duplicate element id {id:?}"),
            id,
        ));
        return;
    }

    let Some(kind) = element.get("kind").and_then(Value::as_str) else {
        diagnostics.push(DocDiagnostic::for_element("missing_kind", "kind must be a string", id));
        return;
    };
    let Some(spec) = models::model_spec(kind) else {
        diagnostics.push(DocDiagnostic::for_element(
            "unsupported_kind",
            format!("unsupported model kind {kind:?}"),
            id,
        ));
        return;
    };

    let Some(nodes) = element.get("nodes").and_then(Value::as_array) else {
        diagnostics.push(DocDiagnostic::for_element(
            "missing_nodes",
            "nodes must be an array of strings",
            id,
        ));
        return;
    };
    if nodes.len() != spec.terminals.len() {
        diagnostics.push(DocDiagnostic::for_element(
            "wrong_terminal_count",
            format!(
                "expected {} terminals for {}, got {}",
                spec.terminals.len(),
                kind,
                nodes.len()
            ),
            id,
        ));
    }
    for node in nodes {
        match node.as_str() {
            Some(node) if !node.is_empty() && node.len() <= MAX_ID_LEN => {}
            _ => {
                diagnostics.push(DocDiagnostic::for_element(
                    "invalid_node",
                    "each node must be a non-empty string of at most 128 chars",
                    id,
                ));
                break;
            }
        }
    }

    if let Some(Value::Object(entries)) = element.get("params") {
        for (key, value) in entries {
            let Some(number) = value.as_f64() else {
                diagnostics.push(DocDiagnostic::for_element(
                    "invalid_param",
                    format!("parameter {key:?} must be a finite number"),
                    id,
                ));
                continue;
            };
            let Some(param) = spec.param(key) else {
                diagnostics.push(DocDiagnostic::for_element(
                    "unknown_param",
                    format!("unknown parameter {key:?} for model {kind:?}"),
                    id,
                ));
                continue;
            };
            if let Some(reason) = models::check_param_value(param, number) {
                diagnostics.push(DocDiagnostic::for_element(
                    "invalid_param",
                    format!("parameter {key:?} {reason}"),
                    id,
                ));
            }
        }
    }

    if let Some(geometry) = element.get("geometry") {
        if geometry.get("flags").is_some_and(|f| f.as_f64().unwrap_or(f64::NAN).fract() != 0.0) {
            diagnostics.push(DocDiagnostic::for_element(
                "invalid_geometry",
                "geometry.flags must be an integer",
                id,
            ));
        }
        for key in ["x1", "y1", "x2", "y2"] {
            match geometry.get(key).and_then(Value::as_f64) {
                Some(value) if value.is_finite() && value.abs() <= 1e6 => {}
                _ => {
                    diagnostics.push(DocDiagnostic::for_element(
                        "invalid_geometry",
                        format!("geometry.{key} must be a finite number within [-1e6, 1e6]"),
                        id,
                    ));
                    break;
                }
            }
        }
    }
}
