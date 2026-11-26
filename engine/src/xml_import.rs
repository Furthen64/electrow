//! Reference-format XML import (CircuitJS1 XML, the reference app's native format).
//!
//! Shape (from reference snapshots):
//!   <circuit f=".." ts=".." ic=".." cb=".." vr=".." pb=".." mts=".." st="..">
//!     <tag x="x1 y1 x2 y2" f=".." ...attrs/>
//!   </circuit>
//!
//! Each supported element maps onto the Electrow component set with the
//! same model semantics as the text import (two-terminal endpoints, BJT/
//! MOSFET post geometry). Unsupported tags (scopes, hints, custom logic,
//! subcircuits, models, adjustables, test data) are collected as
//! `UnsupportedRecord`s, never silently dropped.

use crate::circuit_doc::{CircuitDocument, CircuitElement, CircuitOptions, DocDiagnostic, Geometry, UnsupportedRecord};
use crate::import_common::{assign_nodes, PendingRecord};
use std::collections::HashMap;

const MAX_XML_BYTES: usize = 4 * 1024 * 1024;
const MAX_ELEMENTS: usize = 4096;

#[derive(Clone, Debug)]
struct XmlNode {
    tag: String,
    attrs: Vec<(String, String)>,
    children: Vec<XmlNode>,
}

impl XmlNode {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn attr_f64(&self, name: &str) -> Option<f64> {
        self.attr(name).and_then(|v| v.parse::<f64>().ok())
    }

    fn attr_i64(&self, name: &str) -> Option<i64> {
        self.attr(name).and_then(|v| v.parse::<i64>().ok())
    }
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

struct XmlParser<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> XmlParser<'a> {
    fn skip_ws(&mut self) {
        let mut i = self.pos;
        let bytes = self.text.as_bytes();
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
            i += 1;
        }
        self.pos = i;
    }

    fn rest(&self) -> &'a str {
        &self.text[self.pos..]
    }

    fn fail<T>(&self, code: &'static str, message: impl Into<String>) -> Result<T, DocDiagnostic> {
        Err(DocDiagnostic::err(code, message))
    }

    fn parse_name(&mut self) -> Result<String, DocDiagnostic> {
        let rest = self.rest();
        let mut i = 0;
        let bytes = rest.as_bytes();
        while i < bytes.len() {
            let c = bytes[i];
            if c.is_ascii_alphabetic() || c == b'_' || c == b':' || c == b'.' || c == b'-' {
                i += 1;
            } else {
                break;
            }
        }
        if i == 0 {
            return self.fail("invalid_xml", "expected a name");
        }
        let name = rest[..i].to_string();
        self.pos += i;
        Ok(name)
    }

    fn parse_quoted(&mut self) -> Result<String, DocDiagnostic> {
        let rest = self.rest();
        let quote = rest.chars().next().unwrap_or('\0');
        if quote != '"' && quote != '\'' {
            return self.fail("invalid_xml", "expected a quoted attribute value");
        }
        self.pos += 1;
        let rest = self.rest();
        let end = match rest.find(quote) {
            Some(end) => end,
            None => return self.fail("invalid_xml", "unterminated quoted attribute value"),
        };
        let value = unescape(&rest[..end]);
        self.pos += end + 1;
        Ok(value)
    }

    fn skip_ignorable(&mut self) -> Result<(), DocDiagnostic> {
        loop {
            self.skip_ws();
            let rest = self.rest();
            if rest.starts_with("<?") {
                match rest.find("?>") {
                    Some(end) => {
                        self.pos += end + 2;
                        continue;
                    }
                    None => return self.fail("invalid_xml", "unterminated processing instruction"),
                }
            }
            if rest.starts_with("<!--") {
                match rest.find("-->") {
                    Some(end) => {
                        self.pos += end + 3;
                        continue;
                    }
                    None => return self.fail("invalid_xml", "unterminated comment"),
                }
            }
            if rest.starts_with("<!") {
                match rest.find('>') {
                    Some(end) => {
                        self.pos += end + 1;
                        continue;
                    }
                    None => return self.fail("invalid_xml", "unterminated declaration"),
                }
            }
            return Ok(());
        }
    }

    fn parse_element(&mut self) -> Result<XmlNode, DocDiagnostic> {
        self.skip_ignorable()?;
        let rest = self.rest();
        if !rest.starts_with('<') {
            return self.fail("invalid_xml", "expected an element");
        }
        if rest.starts_with("</") {
            return self.fail("invalid_xml", "unexpected closing tag");
        }
        self.pos += 1;
        let tag = self.parse_name()?;

        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        loop {
            self.skip_ws();
            let rest = self.rest();
            if rest.starts_with("/>") {
                self.pos += 2;
                self_closing = true;
                break;
            }
            if let Some(rest) = rest.strip_prefix('>') {
                let _ = rest;
                self.pos += 1;
                break;
            }
            if rest.starts_with("<") || rest.is_empty() {
                return self.fail("invalid_xml", "expected an attribute name or '>'");
            }
            let name = self.parse_name()?;
            self.skip_ws();
            if !self.rest().starts_with('=') {
                return self.fail("invalid_xml", "expected '=' after attribute name");
            }
            self.pos += 1;
            self.skip_ws();
            let value = self.parse_quoted()?;
            attrs.push((name, value));
        }

        if self_closing {
            return Ok(XmlNode {
                tag,
                attrs,
                children: Vec::new(),
            });
        }

        let mut children = Vec::new();
        loop {
            self.skip_ws();
            let rest = self.rest();
            if rest.is_empty() {
                return self.fail("invalid_xml", "unterminated element: EOF in <{tag}>");
            }
            if rest.starts_with("</") {
                self.pos += 2;
                let closing = self.parse_name()?;
                self.skip_ws();
                if !self.rest().starts_with('>') {
                    return self.fail("invalid_xml", "expected '>' after closing tag");
                }
                self.pos += 1;
                if closing != tag {
                    return self.fail(
                        "invalid_xml",
                        format!("mismatched closing tag </{closing}>, expected </{tag}>"),
                    );
                }
                return Ok(XmlNode { tag, attrs, children });
            }
            if rest.starts_with("<!--") {
                match rest.find("-->") {
                    Some(end) => {
                        self.pos += end + 3;
                        continue;
                    }
                    None => return self.fail("invalid_xml", "unterminated comment"),
                }
            }
            if rest.starts_with('<') {
                let child = self.parse_element()?;
                children.push(child);
                continue;
            }
            // Text content: reference ignores it, as does the text importer.
            let end = rest.find('<').unwrap_or(rest.len());
            self.pos += end;
        }
    }
}

fn parse_document(text: &str) -> Result<XmlNode, DocDiagnostic> {
    let mut parser = XmlParser { text, pos: 0 };
    parser.skip_ignorable()?;
    let rest = parser.rest();
    if rest.is_empty() {
        return Err(DocDiagnostic::err("invalid_xml", "empty document"));
    }
    if !rest.starts_with('<') || rest.starts_with("</") {
        return Err(DocDiagnostic::err("invalid_xml", "expected the root element"));
    }
    parser.parse_element()
}

struct ElementOutcome {
    element: Option<CircuitElement>,
    unsupported: Option<UnsupportedRecord>,
    warnings: Vec<String>,
}

#[derive(Debug)]
pub struct XmlImport {
    pub document: CircuitDocument,
    pub unsupported: Vec<UnsupportedRecord>,
    pub warnings: Vec<String>,
}

fn geometry_from_attr(tag: &str, index: usize, node: &XmlNode) -> Result<Geometry, UnsupportedRecord> {
    let Some(x) = node.attr("x") else {
        return Err(UnsupportedRecord {
            line: index,
            type_token: tag.to_string(),
            reason: "missing position attribute".to_string(),
        });
    };
    let parts: Vec<&str> = x.split_whitespace().collect();
    let points: Vec<i64> = parts
        .iter()
        .map(|p| p.parse::<i64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| UnsupportedRecord {
            line: index,
            type_token: tag.to_string(),
            reason: format!("bad position attribute {x:?}"),
        })?;
    if points.len() != 4 {
        return Err(UnsupportedRecord {
            line: index,
            type_token: tag.to_string(),
            reason: format!("bad position attribute {x:?}"),
        });
    }
    Ok(Geometry {
        x1: points[0],
        y1: points[1],
        x2: points[2],
        y2: points[3],
        flags: node.attr_i64("f").unwrap_or(0) as u32,
    })
}

fn make_element(_tag: &str, kind: &str, geometry: Geometry, params: HashMap<String, f64>) -> CircuitElement {
    CircuitElement {
        id: String::new(),
        kind: kind.to_string(),
        nodes: Vec::new(),
        params,
        geometry: Some(geometry),
    }
}

/// Parse root options attributes (f, ts, ic, cb, vr, pb, mts, st).
/// Mirrors CircuitLoader.readOptions/readCircuitFlags.
fn parse_options(root: &XmlNode) -> Result<(CircuitOptions, Vec<String>), DocDiagnostic> {
    let mut warnings = Vec::new();
    let flags = root.attr_i64("f").unwrap_or(0) as u32;

    let max_time_step_s = match root.attr_f64("ts") {
        Some(ts) => {
            if !ts.is_finite() || ts <= 0.0 || ts > 1.0 {
                return Err(DocDiagnostic::err(
                    "invalid_options",
                    format!("maxTimeStep must be in (0, 1]; got {ts}"),
                ));
            }
            ts
        }
        None => CircuitOptions::default().max_time_step_s,
    };

    let min_time_step_s = root
        .attr_f64("mts")
        .unwrap_or(CircuitOptions::default().min_time_step_s);

    let voltage_range = root.attr_f64("vr").unwrap_or(0.0);
    if voltage_range > 0.0 {
        // keep
    } else if root.attr("vr").is_some() {
        return Err(DocDiagnostic::err(
            "invalid_options",
            format!("voltageRange must be positive; got {voltage_range}"),
        ));
    }

    if root.attr_i64("st").is_some() {
        warnings.push("solver type is not imported".to_string());
    }

    Ok((
        CircuitOptions {
            flags,
            max_time_step_s,
            adjust_time_step: (flags & 64) != 0,
            auto_dc_on_reset: (flags & 128) != 0,
            voltage_range,
            min_time_step_s,
            ..CircuitOptions::default()
        },
        warnings,
    ))
}

fn process_element(tag: &str, index: usize, node: &XmlNode) -> ElementOutcome {
    if !node.children.is_empty() {
        return ElementOutcome {
            element: None,
            unsupported: Some(UnsupportedRecord {
                line: index,
                type_token: tag.to_string(),
                reason: "unexpected nested content".to_string(),
            }),
            warnings: Vec::new(),
        };
    }
    let geometry = match geometry_from_attr(tag, index, node) {
        Ok(geometry) => geometry,
        Err(record) => {
            return ElementOutcome {
                element: None,
                unsupported: Some(record),
                warnings: Vec::new(),
            };
        }
    };
    let flags = geometry.flags;
    let mut params = HashMap::new();
    let mut warnings = Vec::new();

    fn unsupported<T>(line: usize, tag: &str, reason: impl Into<String>) -> Result<T, UnsupportedRecord> {
        Err(UnsupportedRecord {
            line,
            type_token: tag.to_string(),
            reason: reason.into(),
        })
    }

    let built = (|| -> Result<CircuitElement, UnsupportedRecord> {
        match tag {
            "w" => Ok(make_element("w", "wire", geometry, params)),
            "g" => Ok(make_element("g", "ground", geometry, params)),
            "r" => {
                let Some(resistance) = node.attr_f64("r") else {
                    return unsupported(index, tag, "missing resistance attribute");
                };
                params.insert("resistance_ohm".to_string(), resistance);
                Ok(make_element("r", "resistor", geometry, params))
            }
            "v" => {
                let waveform = node.attr_i64("wf").unwrap_or(0);
                if waveform != 0 {
                    return unsupported(index, tag, format!("non-DC waveform ({waveform}) is not supported"));
                }
                let max_voltage = node
                    .attr_f64("maxv")
                    .ok_or_else(|| UnsupportedRecord {
                        line: index,
                        type_token: tag.to_string(),
                        reason: "missing maxv attribute".to_string(),
                    })?;
                let bias = node.attr_f64("bias").unwrap_or(0.0);
                params.insert("voltage_v".to_string(), max_voltage + bias);
                if let Some(ir) = node.attr_f64("ir") {
                    if ir > 0.0 {
                        warnings.push(format!("element {index}: internal resistance is not imported"));
                    }
                }
                if node.attr_f64("fr").is_some() {
                    warnings.push(format!("element {index}: frequency is not imported"));
                }
                Ok(make_element("v", "voltage_source", geometry, params))
            }
            "i" => {
                let Some(current) = node.attr_f64("cu") else {
                    return unsupported(index, tag, "missing current attribute");
                };
                if let Some(max_voltage) = node.attr_f64("mv") {
                    if max_voltage > 0.0 {
                        return unsupported(index, tag, "voltage-limited current source is not supported");
                    }
                }
                params.insert(
                    "current_a".to_string(),
                    if current == 0.0 { 0.01 } else { current },
                );
                Ok(make_element("i", "current_source", geometry, params))
            }
            "c" => {
                let Some(capacitance) = node.attr_f64("c") else {
                    return unsupported(index, tag, "missing capacitance attribute");
                };
                params.insert("capacitance_f".to_string(), capacitance);
                let mut anything = false;
                for attr in ["iv", "sr", "vd"] {
                    if let Some(value) = node.attr_f64(attr) {
                        if value != 0.0 {
                            anything = true;
                        }
                    }
                }
                if anything {
                    warnings.push(format!(
                        "element {index}: capacitor initial state is not imported"
                    ));
                }
                Ok(make_element("c", "capacitor", geometry, params))
            }
            "l" => {
                let Some(inductance) = node.attr_f64("l") else {
                    return unsupported(index, tag, "missing inductance attribute");
                };
                params.insert("inductance_h".to_string(), inductance);
                let mut anything = false;
                for attr in ["ic", "isat", "i"] {
                    if let Some(value) = node.attr_f64(attr) {
                        if value != 0.0 {
                            anything = true;
                        }
                    }
                }
                if anything {
                    warnings.push(format!(
                        "element {index}: inductor initial state is not imported"
                    ));
                }
                Ok(make_element("l", "inductor", geometry, params))
            }
            "d" => {
                if let Some(name) = node.attr("mo") {
                    if name != "default" {
                        return unsupported(index, tag, format!("custom diode model {name:?} is not supported"));
                    }
                }
                Ok(make_element("d", "diode", geometry, params))
            }
            "s" => {
                let position = node.attr_i64("p").unwrap_or(0);
                params.insert("closed".to_string(), if position == 0 { 1.0 } else { 0.0 });
                if let Some(resistance) = node.attr_f64("r") {
                    if resistance.is_finite() && resistance > 0.0 {
                        params.insert("resistance_ohm".to_string(), resistance);
                    } else {
                        warnings.push(format!("element {index}: invalid switch resistance is not imported"));
                    }
                }
                if node.attr("mm").is_some()
                    || node.attr("lab").is_some()
                    || node.attr("key").is_some()
                {
                    warnings.push(format!(
                        "element {index}: switch momentary/label/keyShortcut is not imported"
                    ));
                }
                Ok(make_element("s", "switch", geometry, params))
            }
            "t" => {
                let pnp = node.attr_i64("pn").unwrap_or(1);
                if pnp != -1 && pnp != 1 {
                    return unsupported(index, tag, format!("bad pnp attribute {pnp:?}"));
                }
                params.insert("pnp".to_string(), if pnp == -1 { 1.0 } else { 0.0 });
                let beta = node
                    .attr_f64("be")
                    .ok_or_else(|| UnsupportedRecord {
                        line: index,
                        type_token: tag.to_string(),
                        reason: "missing beta attribute".to_string(),
                    })?;
                params.insert("beta".to_string(), beta);
                if let Some(name) = node.attr("mo") {
                    if name != "default" {
                        return unsupported(index, tag, format!("custom BJT model {name:?} is not supported"));
                    }
                }
                if node.attr("vbe").is_some() || node.attr("vbc").is_some() {
                    warnings.push(format!("element {index}: BJT initial state (vbe/vbc) is not imported"));
                }
                Ok(make_element("t", "bjt", geometry, params))
            }
            "f" => {
                let model = node.attr("mo");
                match model {
                    Some(name) => {
                        if name != "default" {
                            return unsupported(index, tag, format!("custom MOSFET model {name:?} is not supported"));
                        }
                    }
                    None => {
                        // Legacy inline vt/be (pre-model circuit files).
                        let vt = node.attr_f64("vt").unwrap_or(1.5);
                        let beta = node.attr_f64("be").unwrap_or(0.02);
                        params.insert("vt".to_string(), vt);
                        params.insert("beta".to_string(), beta);
                        const FLAG_DIGITAL_LEGACY: u32 = 4;
                        const FLAG_HIDE_BULK_LEGACY: u32 = 16;
                        const FLAG_BODY_DIODE_LEGACY: u32 = 32;
                        const FLAG_BODY_TERMINAL_LEGACY: u32 = 64;
                        const FLAG_SHOW_BODY_DIODE_LEGACY: u32 = 128;
                        if flags & FLAG_BODY_DIODE_LEGACY != 0 {
                            return unsupported(index, tag, "MOSFET body-diode model is not supported");
                        }
                        if flags & FLAG_BODY_TERMINAL_LEGACY != 0 {
                            warnings.push(format!("element {index}: MOSFET body-terminal setting is not imported"));
                        }
                        if flags & (FLAG_DIGITAL_LEGACY | FLAG_HIDE_BULK_LEGACY) != 0 {
                            warnings.push(format!("element {index}: MOSFET bulk/digital display option is not imported"));
                        }
                        if flags & FLAG_SHOW_BODY_DIODE_LEGACY != 0 {
                            warnings.push(format!("element {index}: MOSFET body-diode symbol display is not imported"));
                        }
                    }
                }
                let pnp_flag = (flags & 1) != 0;
                params.insert("pnp".to_string(), if pnp_flag { 1.0 } else { 0.0 });
                Ok(make_element("f", "mosfet", geometry, params))
            }
            _ => {
                return unsupported(index, tag, format!("unsupported element {tag:?}"));
            }
        }
    })();

    match built {
        Ok(element) => ElementOutcome {
            element: Some(element),
            unsupported: None,
            warnings,
        },
        Err(record) => ElementOutcome {
            element: None,
            unsupported: Some(record),
            warnings,
        },
    }
}

pub fn parse_xml_import(text: &str) -> Result<XmlImport, DocDiagnostic> {
    if text.len() > MAX_XML_BYTES {
        return Err(DocDiagnostic::err(
            "document_too_large",
            format!("XML document is {} bytes, limit is {MAX_XML_BYTES}", text.len()),
        ));
    }
    let root = parse_document(text)?;
    if root.tag != "circuit" {
        return Err(DocDiagnostic::err(
            "invalid_xml",
            format!("root element is <{}>; expected <circuit>", root.tag),
        ));
    }
    let (options, mut warnings) = parse_options(&root)?;

    let mut pending = Vec::new();
    let mut unsupported = Vec::new();

    for (index, child) in root.children.iter().enumerate() {
        let outcome = process_element(&child.tag, index + 1, child);
        warnings.extend(outcome.warnings);
        if let Some(record) = outcome.unsupported {
            unsupported.push(record);
            continue;
        }
        let Some(element) = outcome.element else {
            continue;
        };
        if pending.len() + 1 > MAX_ELEMENTS {
            unsupported.push(UnsupportedRecord {
                line: index + 1,
                type_token: child.tag.clone(),
                reason: format!("element limit of {MAX_ELEMENTS} exceeded"),
            });
            continue;
        }
        pending.push(PendingRecord {
            line: index + 1,
            type_token: child.tag.clone(),
            element,
        });
    }

    let (elements, failures) = assign_nodes(pending);
    for failure in failures {
        unsupported.push(UnsupportedRecord {
            line: failure.line,
            type_token: failure.type_token,
            reason: failure.reason,
        });
    }

    let document = CircuitDocument {
        circuit_id: String::new(),
        title: None,
        options,
        elements,
    };

    Ok(XmlImport {
        document,
        unsupported,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT_DIVIDER: &str = "\
$ 1 5.0E-6 10 63 10.0 62
v 112 368 112 48 0 0 0.0 10.0 0.0
w 112 48 240 48 0
r 240 48 240 208 0 10000
r 240 208 240 368 0 10000
w 112 368 240 368 0
";

    const XML_DIVIDER: &str = r#"<circuit f="1" ts="5.0E-6" vr="10.0">
  <v x="112 368 112 48" f="0" wf="0" maxv="10.0" bias="0.0"/>
  <w x="112 48 240 48" f="0"/>
  <r x="240 48 240 208" f="0" r="10000"/>
  <r x="240 208 240 368" f="0" r="10000"/>
  <w x="112 368 240 368" f="0"/>
</circuit>"#;

    #[test]
    fn xml_matches_text_import_for_divider() {
        let text = crate::falstad::parse_falstad_text(TEXT_DIVIDER).unwrap();
        let xml = parse_xml_import(XML_DIVIDER).unwrap();
        assert_eq!(text.document, xml.document);
    }

    #[test]
    fn xml_options_and_elements() {
        let import = parse_xml_import(XML_DIVIDER).unwrap();
        assert_eq!(import.document.options.max_time_step_s, 5.0e-6);
        assert_eq!(import.document.options.voltage_range, 10.0);
        assert_eq!(import.document.options.flags, 1);
        assert_eq!(import.document.elements.len(), 5);
        assert_eq!(import.document.elements[1].kind, "wire");
        assert_eq!(import.document.elements[2].params.get("resistance_ohm"), Some(&10000.0));
    }

    #[test]
    fn xml_unsupported_tag_is_reported() {
        let xml = "<circuit f=\"0\" ts=\"5.0E-6\" vr=\"10\">\n  <o x=\"0 0 10 10\" f=\"0\"/>\n</circuit>";
        let import = parse_xml_import(xml).unwrap();
        assert_eq!(import.document.elements.len(), 0);
        assert_eq!(import.unsupported.len(), 1);
        assert_eq!(import.unsupported[0].type_token, "o");
    }

    #[test]
    fn xml_element_missing_position_is_reported() {
        let xml = "<circuit f=\"0\" ts=\"5.0E-6\" vr=\"10\">\n  <r f=\"0\" r=\"1000\"/>\n</circuit>";
        let import = parse_xml_import(xml).unwrap();
        assert_eq!(import.document.elements.len(), 0);
        assert_eq!(import.unsupported.len(), 1);
        assert!(import.unsupported[0].reason.contains("position"));
    }

    #[test]
    fn xml_malformed_document_rejected() {
        let err = parse_xml_import("<circuit><r x=\"0 0 1 1\" r=\"1000\"/>").unwrap_err();
        assert_eq!(err.code, "invalid_xml");
    }

    #[test]
    fn xml_root_must_be_circuit() {
        let err = parse_xml_import("<not-circuit/>").unwrap_err();
        assert_eq!(err.code, "invalid_xml");
    }

    #[test]
    fn xml_rejects_bad_options() {
        let err = parse_xml_import("<circuit f=\"0\" ts=\"5.0E-6\" vr=\"0\"/>").unwrap_err();
        assert_eq!(err.code, "invalid_options");
    }

    #[test]
    fn xml_rejects_oversized_document() {
        let big = format!("<circuit f=\"0\" ts=\"5.0E-6\" vr=\"10\">{{\"pad\":\"{}\"}}</circuit>", "a".repeat(MAX_XML_BYTES));
        assert!(parse_xml_import(&big).is_err());
    }

    #[test]
    fn xml_bjt_imports_pnp_beta_and_warns_on_state() {
        let xml = "<circuit f=\"0\" ts=\"5.0E-6\" vr=\"10\">\n  <t x=\"100 100 100 200\" f=\"0\" pn=\"-1\" be=\"50\" mo=\"default\" vbe=\"0.6\"/>\n</circuit>";
        let import = parse_xml_import(xml).unwrap();
        assert_eq!(import.document.elements.len(), 1);
        let bjt = &import.document.elements[0];
        assert_eq!(bjt.params.get("pnp"), Some(&1.0));
        assert_eq!(bjt.params.get("beta"), Some(&50.0));
        assert_eq!(bjt.nodes.len(), 3);
        assert!(!import.warnings.is_empty());
    }

    #[test]
    fn xml_bjt_custom_model_is_reported() {
        let xml = "<circuit f=\"0\" ts=\"5.0E-6\" vr=\"10\">\n  <t x=\"100 100 100 200\" f=\"0\" pn=\"1\" be=\"100\" mo=\"custom\"/>\n</circuit>";
        let import = parse_xml_import(xml).unwrap();
        assert_eq!(import.document.elements.len(), 0);
        assert_eq!(import.unsupported.len(), 1);
        assert!(import.unsupported[0].reason.contains("custom BJT model"));
    }

    #[test]
    fn xml_mosfet_modern_model_default() {
        let xml = "<circuit f=\"0\" ts=\"5.0E-6\" vr=\"10\">\n  <f x=\"100 100 100 200\" f=\"1\" mo=\"default\"/>\n</circuit>";
        let import = parse_xml_import(xml).unwrap();
        assert_eq!(import.document.elements.len(), 1);
        let mosfet = &import.document.elements[0];
        assert_eq!(mosfet.params.get("pnp"), Some(&1.0));
        assert_eq!(mosfet.nodes.len(), 3);
    }

    #[test]
    fn xml_mosfet_legacy_vt_be_and_legacy_flags() {
        let xml = "<circuit f=\"0\" ts=\"5.0E-6\" vr=\"10\">\n  <f x=\"100 100 100 200\" f=\"1\" vt=\"2.0\" be=\"0.03\"/>\n</circuit>";
        let import = parse_xml_import(xml).unwrap();
        assert_eq!(import.document.elements.len(), 1);
        let mosfet = &import.document.elements[0];
        assert_eq!(mosfet.params.get("vt"), Some(&2.0));
        assert_eq!(mosfet.params.get("beta"), Some(&0.03));
        assert_eq!(mosfet.params.get("pnp"), Some(&1.0));
    }

    #[test]
    fn xml_mosfet_body_diode_model_is_reported() {
        let xml = "<circuit f=\"0\" ts=\"5.0E-6\" vr=\"10\">\n  <f x=\"100 100 100 200\" f=\"32\" vt=\"1.5\" be=\"0.02\"/>\n</circuit>";
        let import = parse_xml_import(xml).unwrap();
        assert_eq!(import.document.elements.len(), 0);
        assert_eq!(import.unsupported.len(), 1);
    }

    #[test]
    fn xml_switch_imports_position_and_resistance() {
        let xml = "<circuit f=\"0\" ts=\"5.0E-6\" vr=\"10\">\n  <s x=\"0 0 100 0\" f=\"0\" p=\"1\" r=\"0.05\"/>\n</circuit>";
        let import = parse_xml_import(xml).unwrap();
        let switch = &import.document.elements[0];
        assert_eq!(switch.params.get("closed"), Some(&0.0));
        assert_eq!(switch.params.get("resistance_ohm"), Some(&0.05));
    }
}
