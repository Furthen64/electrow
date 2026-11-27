use std::fs;
use std::path::Path;

use electrow_engine::circuit_doc;
use electrow_engine::falstad;
use electrow_engine::json::{self, Value};
use electrow_engine::solver::{Circuit, Diagnostic, Operation};

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    json::parse(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or_else(|| panic!("expected a number, got {value:?}"))
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_else(|| panic!("expected a string, got {value:?}"))
}

fn build_circuit(document: &Value) -> Circuit {
    let parsed = circuit_doc::parse_document(&document.to_json())
        .unwrap_or_else(|diagnostics| panic!("invalid fixture document: {diagnostics:?}"));
    let (operations, params) = circuit_doc::document_operations(&parsed);
    let mut circuit = Circuit::new();
    circuit.apply_operations(operations).unwrap();
    for (element_id, values) in params {
        for (key, value) in values {
            circuit.set_parameter(&element_id, &key, value).unwrap();
        }
    }
    circuit
}

fn node_voltage(circuit: &Circuit, id: &str) -> f64 {
    circuit
        .node_voltages()
        .into_iter()
        .find(|(node, _)| node == id)
        .map(|(_, voltage)| voltage)
        .unwrap_or_else(|| panic!("missing node {id}"))
}

fn actual_for(circuit: &Circuit, check: &Value) -> f64 {
    let id = text(check.get("id").expect("check needs an id"));
    if circuit.node_voltages().iter().any(|(node, _)| node == id) {
        return node_voltage(circuit, id);
    }
    if let Some(quantity) = check.get("quantity").and_then(Value::as_str) {
        return circuit.measurement(id, quantity).expect("missing measurement");
    }
    circuit
        .measurement(id, "current")
        .or_else(|| circuit.measurement(id, "voltage"))
        .expect("missing measurement")
}

fn assert_close(actual: f64, expected: f64, atol: f64, rtol: f64, context: &str) {
    let limit = atol + rtol * expected.abs();
    assert!(
        (actual - expected).abs() <= limit,
        "{context}: actual {actual}, expected {expected}, tolerance {limit}"
    );
}

fn kcl_residual(circuit: &Circuit, node: &str) -> f64 {
    let voltages: std::collections::HashMap<String, f64> =
        circuit.node_voltages().into_iter().collect();
    let at = *voltages.get(node).unwrap_or(&0.0);
    let mut sum = 0.0;
    for element in &circuit.elements {
        if element.kind != "resistor" || element.nodes.len() != 2 {
            continue;
        }
        let resistance = element.params["resistance_ohm"];
        if element.nodes[0] == node {
            sum += (at - voltages.get(&element.nodes[1]).copied().unwrap_or(0.0)) / resistance;
        }
        if element.nodes[1] == node {
            sum += (at - voltages.get(&element.nodes[0]).copied().unwrap_or(0.0)) / resistance;
        }
    }
    sum
}

fn run_fixture(name: &str) {
    let fixture = fixture(name);
    let circuit_value = fixture.get("circuit").unwrap_or(&fixture);
    let mut circuit = build_circuit(circuit_value);
    let settings = fixture.get("settings").unwrap();
    let dt = number(settings.get("dt_s").unwrap());
    let steps = number(settings.get("steps").unwrap()) as u64;
    let checks = fixture.get("checks").and_then(Value::as_array).unwrap();

    let mut peaks: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let track_peaks = checks
        .iter()
        .any(|check| check.get("type").and_then(Value::as_str) == Some("peak"));
    if track_peaks {
        for _ in 0..steps {
            let outcome = circuit.step(dt, 1);
            assert!(!outcome.failed, "{name}: step failed: {:?}", outcome.diagnostics);
            for check in checks.iter().filter(|check| {
                check.get("type").and_then(Value::as_str) == Some("peak")
            }) {
                let id = text(check.get("id").unwrap());
                let value = node_voltage(&circuit, id);
                peaks
                    .entry(id.to_string())
                    .and_modify(|peak| *peak = peak.max(value))
                    .or_insert(value);
            }
        }
    } else {
        let outcome = circuit.step(dt, steps);
        assert!(!outcome.failed, "{name}: step failed: {:?}", outcome.diagnostics);
    }

    for check in checks {
        match text(check.get("type").unwrap()) {
            "node" | "current" => {
                let actual = actual_for(&circuit, check);
                let expected = number(check.get("expected").unwrap());
                let atol = check.get("atol").map(number).unwrap_or(1e-9);
                let rtol = check.get("rtol").map(number).unwrap_or(0.0);
                assert_close(actual, expected, atol, rtol, &format!("{name}/{}", text(check.get("id").unwrap())));
            }
            "range" => {
                let actual = actual_for(&circuit, check);
                let low = number(check.get("min").unwrap());
                let high = number(check.get("max").unwrap());
                assert!((low..=high).contains(&actual), "{name}: {actual} outside [{low}, {high}]");
            }
            "min" => {
                let actual = actual_for(&circuit, check);
                assert!(actual >= number(check.get("value").unwrap()), "{name}: {actual} below minimum");
            }
            "peak" => {
                let id = text(check.get("id").unwrap());
                let low = number(check.get("min").unwrap());
                let high = number(check.get("max").unwrap());
                let peak = peaks.get(id).copied().unwrap_or_else(|| node_voltage(&circuit, id));
                assert!((low..=high).contains(&peak), "{name}: peak {peak} outside [{low}, {high}]");
            }
            "element_count" => {
                let actual = circuit.elements.len() as f64;
                assert_eq!(actual, number(check.get("expected").unwrap()), "{name}: element count");
            }
            other => panic!("{name}: unknown check type {other}"),
        }
    }

    if let Some(kcls) = fixture.get("kcl").and_then(Value::as_array) {
        for entry in kcls {
            let node = text(entry.get("node").unwrap());
            let atol = number(entry.get("atol").unwrap());
            let residual = kcl_residual(&circuit, node);
            assert!(residual.abs() <= atol, "{name}: KCL residual {residual} at {node}");
        }
    }
}

#[test]
fn divider_dc_fixture() {
    run_fixture("01-divider-dc.json");
}

#[test]
fn rc_step_fixture() {
    run_fixture("02-rc-step.json");
}

#[test]
fn rl_step_fixture() {
    run_fixture("03-rl-step.json");
}

#[test]
fn rlc_settling_fixture() {
    run_fixture("04-rlc-settling.json");
}

#[test]
fn diode_forward_fixture() {
    run_fixture("05-diode-forward.json");
}

#[test]
fn bjt_switching_fixture() {
    run_fixture("06-bjt-switching.json");
}

#[test]
fn mosfet_linear_fixture() {
    run_fixture("07-mosfet-linear.json");
}

#[test]
fn vactrol_fixture() {
    run_fixture("10-vactrol.json");
}

#[test]
fn falstad_import_fixtures() {
    for name in ["08-falstad-divider.json", "09-unsupported-import.json"] {
        let fixture = fixture(name);
        let dump = text(fixture.get("dump").unwrap());
        let import = falstad::parse_falstad_text(dump).unwrap();
        for check in fixture.get("checks").and_then(Value::as_array).unwrap() {
            match text(check.get("type").unwrap()) {
                "element_count" => assert_eq!(
                    import.document.elements.len() as f64,
                    number(check.get("expected").unwrap()),
                    "{name}: element count"
                ),
                "unsupported_count" => assert_eq!(
                    import.unsupported.len() as f64,
                    number(check.get("expected").unwrap()),
                    "{name}: unsupported count"
                ),
                "unsupported_token" => assert!(import
                    .unsupported
                    .iter()
                    .any(|record| record.type_token == text(check.get("expected").unwrap()))),
                "node_shared" => {
                    let a = import
                        .document
                        .elements
                        .iter()
                        .find(|element| element.id == text(check.get("a").unwrap()))
                        .unwrap();
                    let b = import
                        .document
                        .elements
                        .iter()
                        .find(|element| element.id == text(check.get("b").unwrap()))
                        .unwrap();
                    let a_index = number(check.get("a_index").unwrap()) as usize;
                    let b_index = number(check.get("b_index").unwrap()) as usize;
                    assert_eq!(a.nodes[a_index], b.nodes[b_index], "{name}: shared node");
                }
                other => panic!("{name}: unknown check {other}"),
            }
        }
    }
}

#[test]
fn document_round_trip_fixtures() {
    for name in [
        "01-divider-dc.json",
        "02-rc-step.json",
        "04-rlc-settling.json",
        "07-mosfet-linear.json",
        "10-vactrol.json",
    ] {
        let fixture = fixture(name);
        let document = circuit_doc::parse_document(&fixture.get("circuit").unwrap().to_json()).unwrap();
        let serialized = circuit_doc::document_to_json(&document).to_json();
        let reparsed = circuit_doc::parse_document(&serialized).unwrap();
        assert_eq!(document, reparsed, "{name}: document round trip");
    }
}

#[test]
fn determinism_reset_and_failed_steps() {
    let first = fixture("01-divider-dc.json");
    let mut a = build_circuit(first.get("circuit").unwrap());
    let mut b = build_circuit(first.get("circuit").unwrap());
    a.step(5e-6, 20);
    b.step(5e-6, 20);
    assert_eq!(a.node_voltages(), b.node_voltages());
    assert_eq!(a.time_s(), b.time_s());

    a.reset();
    assert_eq!(a.time_s(), 0.0);
    assert_eq!(a.current_dt(), electrow_engine::solver::DEFAULT_DT_S);
    assert!(a.node_voltages().iter().all(|(_, voltage)| *voltage == 0.0));

    let diode = fixture("05-diode-forward.json");
    let mut refused = build_circuit(diode.get("circuit").unwrap());
    refused.settings.max_iterations = 1;
    let snapshot = refused.node_voltages();
    let outcome = refused.step(5e-6, 1);
    assert!(outcome.failed);
    assert!(outcome
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic, Diagnostic::NonConvergence { .. })));
    assert_eq!(refused.node_voltages(), snapshot);
    assert_eq!(refused.time_s(), 0.0);
}

#[test]
fn topology_edits_and_invalid_parameters_are_reported() {
    let fixture = fixture("01-divider-dc.json");
    let mut circuit = build_circuit(fixture.get("circuit").unwrap());
    assert!(circuit.diagnostics.is_empty());
    circuit
        .apply_operations(vec![
            Operation::Remove {
                id: "v1".to_string(),
            },
            Operation::Remove {
                id: "r2".to_string(),
            },
        ])
        .unwrap();
    assert!(circuit
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic, Diagnostic::FloatingNode { .. })));
    assert!(circuit.step(5e-6, 1).failed);
    assert!(matches!(
        circuit.set_parameter("r1", "resistance_ohm", 0.0),
        Err(Diagnostic::InvalidValue { .. })
    ));
}
