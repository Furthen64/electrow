//! Stable JSON serialization (deterministic key and param order).

use super::types::{CircuitDocument, CircuitElement, CircuitOptions, Geometry, CIRCUIT_DOC_FORMAT, CIRCUIT_DOC_VERSION};
use crate::json::Value;
use std::collections::HashMap;

fn write_params(params: &HashMap<String, f64>) -> Value {
    let mut entries: Vec<(String, Value)> = params
        .iter()
        .map(|(key, value)| (key.clone(), Value::Number(*value)))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Value::Object(entries)
}

fn write_geometry(geometry: &Geometry) -> Value {
    Value::Object(vec![
        ("x1".to_string(), Value::Number(geometry.x1 as f64)),
        ("y1".to_string(), Value::Number(geometry.y1 as f64)),
        ("x2".to_string(), Value::Number(geometry.x2 as f64)),
        ("y2".to_string(), Value::Number(geometry.y2 as f64)),
        ("flags".to_string(), Value::Number(geometry.flags as f64)),
    ])
}

fn write_options(options: &CircuitOptions) -> Value {
    Value::Object(vec![
        ("flags".to_string(), Value::Number(options.flags as f64)),
        ("max_time_step_s".to_string(), Value::Number(options.max_time_step_s)),
        ("min_time_step_s".to_string(), Value::Number(options.min_time_step_s)),
        ("adjust_time_step".to_string(), Value::Bool(options.adjust_time_step)),
        ("auto_dc_on_reset".to_string(), Value::Bool(options.auto_dc_on_reset)),
        ("voltage_range".to_string(), Value::Number(options.voltage_range)),
    ])
}

fn write_element(element: &CircuitElement) -> Value {
    let mut entries = vec![
        ("id".to_string(), Value::String(element.id.clone())),
        ("kind".to_string(), Value::String(element.kind.clone())),
        (
            "nodes".to_string(),
            Value::Array(element.nodes.iter().cloned().map(Value::String).collect()),
        ),
        ("params".to_string(), write_params(&element.params)),
    ];
    if let Some(geometry) = &element.geometry {
        entries.push(("geometry".to_string(), write_geometry(geometry)));
    }
    Value::Object(entries)
}

pub fn document_to_json(document: &CircuitDocument) -> Value {
    let mut entries = vec![
        ("format".to_string(), Value::String(CIRCUIT_DOC_FORMAT.to_string())),
        ("version".to_string(), Value::Number(CIRCUIT_DOC_VERSION as f64)),
        ("circuit_id".to_string(), Value::String(document.circuit_id.clone())),
        ("options".to_string(), write_options(&document.options)),
        (
            "elements".to_string(),
            Value::Array(document.elements.iter().map(write_element).collect()),
        ),
    ];
    if let Some(title) = &document.title {
        entries.push(("title".to_string(), Value::String(title.clone())));
    }
    Value::Object(entries)
}
