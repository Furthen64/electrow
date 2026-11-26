//! Per-kind parameter mapping: raw tokens → Electrow component parameters.

use crate::circuit_doc::{CircuitElement, Geometry};
use crate::falstad::UnsupportedRecord;
use crate::falstad::parser::RawElement;
use std::collections::HashMap;

pub(crate) fn map_kind(raw: &RawElement) -> Option<&'static str> {
    match raw.type_token.as_str() {
        "w" => Some("wire"),
        "g" => Some("ground"),
        "r" => Some("resistor"),
        "v" => Some("voltage_source"),
        "i" => Some("current_source"),
        "c" => Some("capacitor"),
        "l" => Some("inductor"),
        "d" => Some("diode"),
        "t" => Some("bjt"),
        "f" => Some("mosfet"),
        "s" => Some("switch"),
        _ => None,
    }
}

pub(crate) fn make_unsupported(
    raw: &RawElement,
    reason: impl Into<String>,
) -> UnsupportedRecord {
    UnsupportedRecord {
        line: raw.line,
        type_token: raw.type_token.clone(),
        reason: reason.into(),
    }
}

fn parse_f64_token(
    raw: &RawElement,
    tokens: &[String],
    index: usize,
    field: &str,
) -> Result<f64, UnsupportedRecord> {
    let Some(token) = tokens.get(index) else {
        return Err(make_unsupported(raw, format!("missing token for {field}")));
    };
    token
        .parse::<f64>()
        .map_err(|_| make_unsupported(raw, format!("bad value for {field}: {token:?}")))
}

pub(crate) fn parse_element(
    raw: &RawElement,
    warnings: &mut Vec<String>,
) -> Result<CircuitElement, UnsupportedRecord> {
    let kind = match map_kind(raw) {
        Some(kind) => kind,
        None => {
            return Err(make_unsupported(
                raw,
                format!("unknown dump type {:?}", raw.type_token),
            ));
        }
    };

    let geometry = Geometry {
        x1: raw.x1,
        y1: raw.y1,
        x2: raw.x2,
        y2: raw.y2,
        flags: raw.flags,
    };
    let mut params: HashMap<String, f64> = HashMap::new();

    match kind {
        "wire" | "ground" => {}
        "resistor" => {
            let resistance = parse_f64_token(raw, &raw.tokens, 0, "resistance")?;
            params.insert("resistance_ohm".to_string(), resistance);
        }
        "voltage_source" => {
            // Legacy files may store only waveform/frequency/maxVoltage/bias.
            if raw.tokens.len() < 4 {
                return Err(make_unsupported(
                    raw,
                    "voltage source dump is missing waveform/frequency/maxVoltage/bias",
                ));
            }
            let waveform: i64 = raw.tokens[0].parse().map_err(|_| {
                make_unsupported(raw, format!("bad waveform token {:?}", raw.tokens[0]))
            })?;
            if waveform != 0 {
                return Err(make_unsupported(
                    raw,
                    format!("non-DC waveform ({waveform}) is not supported"),
                ));
            }
            let max_voltage = parse_f64_token(raw, &raw.tokens, 2, "maxVoltage")?;
            let bias = parse_f64_token(raw, &raw.tokens, 3, "bias")?;
            params.insert("voltage_v".to_string(), max_voltage + bias);
        }
        "current_source" => {
            let current = parse_f64_token(raw, &raw.tokens, 0, "current")?;
            let max_voltage = parse_f64_token(raw, &raw.tokens, 1, "maxVoltage")?;
            if max_voltage > 0.0 {
                return Err(make_unsupported(
                    raw,
                    "voltage-limited current source is not supported",
                ));
            }
            params.insert(
                "current_a".to_string(),
                if current == 0.0 { 0.01 } else { current },
            );
        }
        "capacitor" => {
            let capacitance = parse_f64_token(raw, &raw.tokens, 0, "capacitance")?;
            params.insert("capacitance_f".to_string(), capacitance);
            warnings.push(format!(
                "line {}: capacitor initial state (voltdiff/initialVoltage/seriesResistance) is not imported",
                raw.line
            ));
        }
        "inductor" => {
            let inductance = parse_f64_token(raw, &raw.tokens, 0, "inductance")?;
            params.insert("inductance_h".to_string(), inductance);
            warnings.push(format!(
                "line {}: inductor initial state/saturation is not imported",
                raw.line
            ));
        }
        "diode" => {
            let model_flag = (raw.flags & 2) != 0;
            let fwdrop_flag = (raw.flags & 1) != 0;
            if model_flag {
                let model_name = raw.tokens.first().cloned().unwrap_or_else(|| "default".to_string());
                let model_name = model_name.replace("%20", " ");
                if model_name != "default" {
                    return Err(make_unsupported(
                        raw,
                        format!("custom diode model {model_name:?} is not supported"),
                    ));
                }
            } else if fwdrop_flag {
                warnings.push(format!("line {}: diode FW-drop is not imported", raw.line));
            }
        }
        "bjt" => {
            let pnp_token = raw.tokens.first().cloned().unwrap_or_else(|| "1".to_string());
            let pnp: i64 = pnp_token.parse().map_err(|_| {
                make_unsupported(raw, format!("bad pnp token {pnp_token:?}"))
            })?;
            params.insert("pnp".to_string(), if pnp == -1 { 1.0 } else { 0.0 });
            // Canonical dump: [pnp, vbe, vbc, beta, modelName?]
            // Legacy/short dump: [pnp, beta] or [pnp].
            let rest = &raw.tokens[1..];
            let numeric_count = rest
                .iter()
                .filter(|token| token.parse::<f64>().is_ok())
                .count();
            if rest.len() >= 3 && numeric_count >= 3 {
                // [vbe, vbc, beta, ...]
                if let Ok(beta) = rest[2].parse::<f64>() {
                    params.insert("beta".to_string(), beta);
                }
            } else if let Some(token) = rest.first() {
                if let Ok(beta) = token.parse::<f64>() {
                    params.insert("beta".to_string(), beta);
                }
            }
            // Model name is a trailing non-numeric token.
            if let Some(tail) = rest.last() {
                if tail.parse::<f64>().is_err() {
                    let tail = tail.replace("%20", " ");
                    if tail != "default" {
                        return Err(make_unsupported(
                            raw,
                            format!("custom BJT model {tail:?} is not supported"),
                        ));
                    }
                }
            }
            if rest.len() >= 2 {
                warnings.push(format!(
                    "line {}: BJT initial state (last vbe/vbc) is not imported",
                    raw.line
                ));
            }
        }
        "mosfet" => {
            let vt = parse_f64_token(raw, &raw.tokens, 0, "vt")?;
            let beta = parse_f64_token(raw, &raw.tokens, 1, "beta")?;
            params.insert("vt".to_string(), vt);
            params.insert("beta".to_string(), beta);
            let pnp_flag = (raw.flags & 1) != 0;
            params.insert("pnp".to_string(), if pnp_flag { 1.0 } else { 0.0 });
        }
        "switch" => {
            let position_token = raw.tokens.first().cloned().unwrap_or_else(|| "0".to_string());
            let position: i64 = match position_token.as_str() {
                "true" => 1,
                "false" => 0,
                other => other.parse().map_err(|_| {
                    make_unsupported(raw, format!("bad switch position token {other:?}"))
                })?,
            };
            params.insert("closed".to_string(), if position == 0 { 1.0 } else { 0.0 });
            if raw.tokens.len() > 1 {
                warnings.push(format!("line {}: switch momentary/label is not imported", raw.line));
            }
        }
        _ => unreachable!(),
    }

    Ok(CircuitElement {
        id: String::new(), // assigned by caller
        kind: kind.to_string(),
        nodes: Vec::new(),
        params,
        geometry: Some(geometry),
    })
}
