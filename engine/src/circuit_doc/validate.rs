//! Schema validation (format, version, ids, options, elements, params).

use super::types::{CircuitOptions, DocDiagnostic, MAX_ELEMENTS, MAX_ID_LEN, MAX_TITLE_LEN};
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

/// Validate already-parsed options, shared by the Falstad text and XML importers.
pub fn validate_circuit_options(options: &CircuitOptions) -> Result<(), DocDiagnostic> {
    if !options.max_time_step_s.is_finite()
        || options.max_time_step_s <= 0.0
        || options.max_time_step_s > 1.0
    {
        return Err(DocDiagnostic::err(
            "invalid_options",
            "max_time_step_s must be in (0, 1]",
        ));
    }
    if !options.min_time_step_s.is_finite()
        || options.min_time_step_s <= 0.0
        || options.min_time_step_s > options.max_time_step_s
    {
        return Err(DocDiagnostic::err(
            "invalid_options",
            "min_time_step_s must be in (0, max_time_step_s]",
        ));
    }
    if !options.voltage_range.is_finite()
        || options.voltage_range <= 0.0
        || options.voltage_range > 1e6
    {
        return Err(DocDiagnostic::err(
            "invalid_options",
            "voltage_range must be in (0, 1e6]",
        ));
    }
    Ok(())
}

fn validate_options(value: Option<&Value>, diagnostics: &mut Vec<DocDiagnostic>) {
    let Some(value) = value else { return };
    if !matches!(value, Value::Object(_)) {
        diagnostics.push(DocDiagnostic::err("invalid_options", "options must be an object"));
        return;
    }
    for key in [
        "max_time_step_s",
        "min_time_step_s",
        "voltage_range",
        "flags",
    ] {
        if let Some(field) = value.get(key) {
            if field.as_f64().is_none() {
                diagnostics.push(DocDiagnostic::err(
                    "invalid_options",
                    format!("options.{key} must be a number"),
                ));
            }
        }
    }
    for key in ["adjust_time_step", "auto_dc_on_reset"] {
        if let Some(field) = value.get(key) {
            if !matches!(field, Value::Bool(_)) {
                diagnostics.push(DocDiagnostic::err(
                    "invalid_options",
                    format!("options.{key} must be a boolean"),
                ));
            }
        }
    }
    if let Some(flags) = value.get("flags").and_then(Value::as_u64) {
        if flags > u32::MAX as u64 {
            diagnostics.push(DocDiagnostic::err(
                "invalid_options",
                "options.flags must fit in u32",
            ));
        }
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

    if let Some(params) = element.get("params") {
        if !matches!(params, Value::Object(_)) {
            diagnostics.push(DocDiagnostic::for_element(
                "invalid_params",
                "params must be an object",
                id,
            ));
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
        if !matches!(geometry, Value::Object(_)) {
            diagnostics.push(DocDiagnostic::for_element(
                "invalid_geometry",
                "geometry must be an object",
                id,
            ));
            return;
        }
        if let Some(flags) = geometry.get("flags") {
            match flags.as_u64() {
                Some(flags) if flags <= u32::MAX as u64 => {}
                _ => diagnostics.push(DocDiagnostic::for_element(
                    "invalid_geometry",
                    "geometry.flags must be a non-negative integer within u32 range",
                    id,
                )),
            }
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
