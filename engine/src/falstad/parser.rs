//! Line-level splitting and structural parsing of the text dump.

use crate::circuit_doc::{CircuitOptions, DocDiagnostic};

pub(crate) const MAX_TEXT_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_LINE_LEN: usize = 4096;
pub(crate) const MAX_ELEMENTS: usize = 4096;

#[derive(Clone, Debug)]
pub(crate) struct RawElement {
    pub line: usize,
    pub type_token: String,
    pub x1: i64,
    pub y1: i64,
    pub x2: i64,
    pub y2: i64,
    pub flags: u32,
    pub tokens: Vec<String>,
}

pub(crate) fn split_tokens(line: &str) -> Vec<String> {
    line.split(|c: char| c.is_whitespace() || c == '+')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn token_is_header_type(type_token: &str) -> bool {
    // Single-character ids never exceed one char; numeric ids are longer.
    type_token.len() > 1 && type_token.chars().all(|c| c.is_ascii_digit())
}

pub(crate) fn parse_options_line(
    tokens: &Vec<String>,
    line: usize,
) -> Result<CircuitOptions, DocDiagnostic> {
    // tokens = the full $ line: ["$", flags, maxTimeStep, speed, currentBar, voltageRange, ...]
    if tokens.len() < 6 {
        return Err(DocDiagnostic::err(
            "invalid_options_line",
            format!("line {line}: expected $ flags maxTimeStep speed currentBar voltageRange [powerBar [minTimeStep]]"),
        ));
    }
    let flags = tokens[1].parse::<u32>().map_err(|_| {
        DocDiagnostic::err(
            "invalid_options_line",
            format!("line {line}: bad flags token {:?}", tokens[1]),
        )
    })?;
    let max_time_step_s = tokens[2].parse::<f64>().map_err(|_| {
        DocDiagnostic::err(
            "invalid_options_line",
            format!("line {line}: bad maxTimeStep token {:?}", tokens[2]),
        )
    })?;
    // tokens[3] = speed (Falstad "iter count" style value), not stored.
    // tokens[4] = currentBar (UI scale), not stored.
    let mut options = CircuitOptions {
        flags,
        max_time_step_s,
        adjust_time_step: (flags & 64) != 0,
        auto_dc_on_reset: (flags & 128) != 0,
        voltage_range: 0.0,
        ..CircuitOptions::default()
    };
    options.voltage_range = tokens[5].parse::<f64>().map_err(|_| {
        DocDiagnostic::err(
            "invalid_options_line",
            format!("line {line}: bad voltageRange token {:?}", tokens[5]),
        )
    })?;
    // tokens[6] = powerBar, tokens[7] = minTimeStep; both optional.
    if tokens.len() >= 8 {
        options.min_time_step_s = tokens[7].parse::<f64>().map_err(|_| {
            DocDiagnostic::err(
                "invalid_options_line",
                format!("line {line}: bad minTimeStep token {:?}", tokens[7]),
            )
        })?;
    } else {
        options.min_time_step_s = CircuitOptions::default().min_time_step_s;
    }
    if let Err(diagnostic) = crate::circuit_doc::validate_circuit_options(&options) {
        return Err(DocDiagnostic::err(
            "invalid_options_line",
            format!("line {line}: {}", diagnostic.message),
        ));
    }
    Ok(options)
}

pub(crate) fn parse_element_line(
    line_no: usize,
    line: &str,
) -> Result<Option<RawElement>, DocDiagnostic> {
    let tokens = split_tokens(line);
    if tokens.is_empty() {
        return Ok(None);
    }
    let type_token = tokens[0].clone();
    // Special tokens that are never elements (afilter/hint/custom-logic lines).
    if type_token == "!" || type_token == "%" || type_token == "?" || type_token == "B" {
        return Ok(None);
    }
    if type_token == "h" || type_token == "o" || type_token == "." {
        return Err(DocDiagnostic::err(
            "unsupported_record",
            format!("line {line_no}: record kind {type_token:?} is not supported"),
        ));
    }
    if token_is_header_type(&type_token) {
        // Numeric type id: either a scope (403) or other unsupported ids.
        return Err(DocDiagnostic::err(
            "unsupported_record",
            format!("line {line_no}: numeric type id {type_token:?} is not supported"),
        ));
    }
    if tokens.len() < 6 {
        return Err(DocDiagnostic::err(
            "malformed_line",
            format!("line {line_no}: expected type x1 y1 x2 y2 flags [params...]"),
        ));
    }
    let parse_i64 = |token: &str, field: &str| -> Result<i64, DocDiagnostic> {
        token.parse::<i64>().map_err(|_| {
            DocDiagnostic::err(
                "malformed_line",
                format!("line {line_no}: bad {field} token {token:?}"),
            )
        })
    };
    let x1 = parse_i64(&tokens[1], "x1")?;
    let y1 = parse_i64(&tokens[2], "y1")?;
    let x2 = parse_i64(&tokens[3], "x2")?;
    let y2 = parse_i64(&tokens[4], "y2")?;
    let flags = tokens[5].parse::<u32>().map_err(|_| {
        DocDiagnostic::err(
            "malformed_line",
            format!("line {line_no}: bad flags token {:?}", tokens[5]),
        )
    })?;
    Ok(Some(RawElement {
        line: line_no,
        type_token,
        x1,
        y1,
        x2,
        y2,
        flags,
        tokens: tokens[6..].to_vec(),
    }))
}

