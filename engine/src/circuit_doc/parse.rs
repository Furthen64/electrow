//! JSON value → typed document (unvalidated; callers validate).

use super::types::{
    CircuitDocument, CircuitElement, CircuitOptions, Geometry, MAX_DOCUMENT_BYTES,
};
use super::types::DocDiagnostic;
use crate::json;
use std::collections::HashMap;

pub fn parse_document(text: &str) -> Result<CircuitDocument, Vec<DocDiagnostic>> {
    if text.len() > MAX_DOCUMENT_BYTES {
        return Err(vec![DocDiagnostic::err(
            "document_too_large",
            format!("document is {} bytes, limit is {}", text.len(), MAX_DOCUMENT_BYTES),
        )]);
    }
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(message) => {
            return Err(vec![DocDiagnostic::err("invalid_json", message)]);
        }
    };
    let mut diagnostics = Vec::new();
    super::validate::parse_document_value(&value, &mut diagnostics);
    if diagnostics.is_empty() {
        Ok(build_document(&value))
    } else {
        Err(diagnostics)
    }
}

fn build_document(value: &json::Value) -> CircuitDocument {
    let circuit_id = value
        .get("circuit_id")
        .and_then(json::Value::as_str)
        .unwrap_or("")
        .to_string();
    let title = value
        .get("title")
        .and_then(json::Value::as_str)
        .map(|s| s.to_string());
    let options = parse_options_value(value.get("options"));
    let mut elements = Vec::new();
    if let Some(values) = value.get("elements").and_then(json::Value::as_array) {
        for element in values {
            if let Some(parsed) = parse_element_value(element) {
                elements.push(parsed);
            }
        }
    }
    CircuitDocument {
        circuit_id,
        title,
        options,
        elements,
    }
}

fn parse_options_value(value: Option<&json::Value>) -> CircuitOptions {
    let mut options = CircuitOptions::default();
    let Some(obj) = value else {
        return options;
    };
    if !matches!(obj, json::Value::Object(_)) {
        return options;
    }
    let get_num = |key: &str| obj.get(key).and_then(json::Value::as_f64);
    if let Some(flags) = obj.get("flags").and_then(json::Value::as_u64) {
        options.flags = flags as u32;
    }
    if let Some(v) = get_num("max_time_step_s") {
        options.max_time_step_s = v;
    }
    if let Some(v) = get_num("min_time_step_s") {
        options.min_time_step_s = v;
    }
    if let Some(v) = obj.get("adjust_time_step").and_then(|v| match v {
        json::Value::Bool(b) => Some(*b),
        _ => None,
    }) {
        options.adjust_time_step = v;
    }
    if let Some(v) = obj.get("auto_dc_on_reset").and_then(|v| match v {
        json::Value::Bool(b) => Some(*b),
        _ => None,
    }) {
        options.auto_dc_on_reset = v;
    }
    if let Some(v) = get_num("voltage_range") {
        options.voltage_range = v;
    }
    options
}

fn parse_element_value(value: &json::Value) -> Option<CircuitElement> {
    let id = value.get("id").and_then(json::Value::as_str)?;
    let kind = value.get("kind").and_then(json::Value::as_str)?;
    let nodes = value
        .get("nodes")
        .and_then(json::Value::as_array)?
        .iter()
        .filter_map(|n| n.as_str().map(|s| s.to_string()))
        .collect();
    let mut params = HashMap::new();
    if let Some(json::Value::Object(entries)) = value.get("params") {
        for (key, val) in entries {
            if let Some(number) = val.as_f64() {
                params.insert(key.clone(), number);
            }
        }
    }
    let geometry = value.get("geometry").and_then(|g| {
        let geo = |key: &str| g.get(key).and_then(json::Value::as_f64);
        let x1 = geo("x1")?;
        let y1 = geo("y1")?;
        let x2 = geo("x2")?;
        let y2 = geo("y2")?;
        Some(Geometry {
            x1: x1 as i64,
            y1: y1 as i64,
            x2: x2 as i64,
            y2: y2 as i64,
            flags: g
                .get("flags")
                .and_then(json::Value::as_u64)
                .unwrap_or(0) as u32,
        })
    });
    Some(CircuitElement {
        id: id.to_string(),
        kind: kind.to_string(),
        nodes,
        params,
        geometry,
    })
}
