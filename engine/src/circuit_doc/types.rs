//! Shared document types and constants.

use crate::json::Value;
use std::collections::HashMap;

pub const CIRCUIT_DOC_FORMAT: &str = "electrow-circuit";
pub const CIRCUIT_DOC_VERSION: u32 = 1;
pub const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
pub const MAX_ELEMENTS: usize = 4096;
pub const MAX_ID_LEN: usize = 128;
pub const MAX_TITLE_LEN: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    pub x1: i64,
    pub y1: i64,
    pub x2: i64,
    pub y2: i64,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CircuitOptions {
    pub flags: u32,
    pub max_time_step_s: f64,
    pub min_time_step_s: f64,
    pub adjust_time_step: bool,
    pub auto_dc_on_reset: bool,
    pub voltage_range: f64,
}

impl Default for CircuitOptions {
    fn default() -> Self {
        Self {
            flags: 0,
            max_time_step_s: 5e-6,
            min_time_step_s: 50e-12,
            adjust_time_step: false,
            auto_dc_on_reset: false,
            voltage_range: 5.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CircuitElement {
    pub id: String,
    pub kind: String,
    pub nodes: Vec<String>,
    pub params: HashMap<String, f64>,
    pub geometry: Option<Geometry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CircuitDocument {
    pub circuit_id: String,
    pub title: Option<String>,
    pub options: CircuitOptions,
    pub elements: Vec<CircuitElement>,
}

impl Default for CircuitDocument {
    fn default() -> Self {
        Self {
            circuit_id: String::new(),
            title: None,
            options: CircuitOptions::default(),
            elements: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DocDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub element_id: Option<String>,
}

impl DocDiagnostic {
    pub fn err(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            element_id: None,
        }
    }

    pub fn for_element(code: &'static str, message: impl Into<String>, element_id: &str) -> Self {
        Self {
            code,
            message: message.into(),
            element_id: Some(element_id.to_string()),
        }
    }

    pub fn to_json(&self) -> Value {
        let mut entries = vec![
            ("code".to_string(), Value::String(self.code.to_string())),
            ("message".to_string(), Value::String(self.message.clone())),
        ];
        if let Some(id) = &self.element_id {
            entries.push(("element_id".to_string(), Value::String(id.clone())));
        }
        Value::Object(entries)
    }
}

/// A reference-format record that could not be imported.
///
/// Unsupported records are collected during import (never silently
/// dropped) and surfaced to the caller with record context.
#[derive(Clone, Debug, PartialEq)]
pub struct UnsupportedRecord {
    pub line: usize,
    pub type_token: String,
    pub reason: String,
}
