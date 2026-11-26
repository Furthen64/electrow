pub mod json;
pub mod solver;

use json::Value;
use solver::{Circuit, Operation};

pub const PROTOCOL_VERSION: u32 = 1;
pub const ENGINE_VERSION: &str = "0.2.0";
pub const MAX_STEP_SECONDS: f64 = 1.0;
pub const MAX_STEPS: u64 = 10_000;
pub const MAX_SUBSCRIBED_SIGNALS: usize = 64;

#[derive(Clone)]
pub struct Signal {
    pub id: String,
    pub element_id: String,
    pub quantity: String,
    pub unit: String,
}

#[derive(Clone)]
pub struct Subscription {
    pub stream_id: String,
    pub signals: Vec<Signal>,
}

pub struct Engine {
    circuit_id: String,
    revision: u64,
    sequence: u64,
    running: bool,
    status: String,
    circuit: Circuit,
    subscriptions: Vec<Subscription>,
    loaded_dump: Option<String>,
}

fn obj(entries: &[(&str, Value)]) -> Value {
    Value::Object(
        entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect(),
    )
}

fn num(value: f64) -> Value {
    Value::Number(value)
}

fn text(value: impl Into<String>) -> Value {
    Value::String(value.into())
}

fn ok(id: &str, result: Value) -> Value {
    obj(&[
        ("v", num(PROTOCOL_VERSION as f64)),
        ("id", text(id)),
        ("ok", Value::Bool(true)),
        ("result", result),
    ])
}

fn fail(id: &str, code: &str, message: impl Into<String>) -> Value {
    obj(&[
        ("v", num(PROTOCOL_VERSION as f64)),
        ("id", text(id)),
        ("ok", Value::Bool(false)),
        (
            "error",
            obj(&[
                ("code", text(code)),
                ("message", text(message)),
                ("retryable", Value::Bool(false)),
            ]),
        ),
    ])
}

impl Engine {
    pub fn new() -> Self {
        Self {
            circuit_id: String::new(),
            revision: 0,
            sequence: 0,
            running: false,
            status: "ready".to_string(),
            circuit: Circuit::new(),
            subscriptions: Vec::new(),
            loaded_dump: None,
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn simulation_time_s(&self) -> f64 {
        self.circuit.time_s()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn handle(&mut self, request: &Value) -> Value {
        let id = request
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("invalid")
            .to_string();

        if request.get("v").and_then(Value::as_f64) != Some(PROTOCOL_VERSION as f64) {
            return fail(
                &id,
                "unsupported_version",
                format!("expected protocol version {}", PROTOCOL_VERSION),
            );
        }

        let cmd = match request.get("cmd").and_then(Value::as_str) {
            Some(cmd) => cmd,
            None => return fail(&id, "bad_request", "cmd must be a string"),
        };

        let empty_args = Value::Object(Vec::new());
        let args = request.get("args").unwrap_or(&empty_args);
        if !matches!(args, Value::Object(_)) {
            return fail(&id, "bad_request", "args must be an object");
        }

        if cmd != "hello" && cmd != "create" && cmd != "shutdown" && self.circuit_id.is_empty() {
            return fail(&id, "bad_request", "no active circuit; call create first");
        }

        match cmd {
            "hello" => ok(
                &id,
                obj(&[
                    ("engine_version", text(ENGINE_VERSION)),
                    ("protocol", num(PROTOCOL_VERSION as f64)),
                    (
                        "capabilities",
                        Value::Array(vec![
                            text("versioned-edits-v1"),
                            text("replaceable-snapshots-v1"),
                            text("bounded-subscriptions-v1"),
                            text("dense-mna-solver-v1"),
                        ]),
                    ),
                ]),
            ),
            "create" => self.create(&id, args),
            "load_circuit" => self.load_circuit(&id, args),
            "topology_edit" => self.topology_edit(&id, args),
            "parameter_edit" => self.parameter_edit(&id, args),
            "reset" => self.reset(&id, args),
            "run" => self.set_running(&id, args, true),
            "pause" => self.set_running(&id, args, false),
            "step" => self.step(&id, args),
            "subscribe" => self.subscribe(&id, args),
            "unsubscribe" => self.unsubscribe(&id, args),
            "snapshot" => ok(&id, self.snapshot()),
            "shutdown" => ok(&id, self.ack(&id)),
            _ => fail(&id, "unknown_command", format!("unknown command: {cmd}")),
        }
    }

    fn ack(&self, _id: &str) -> Value {
        obj(&[
            ("type", text("ack")),
            ("circuit_id", text(self.circuit_id.clone())),
            ("revision", num(self.revision as f64)),
            ("sequence", num(self.sequence as f64)),
        ])
    }

    fn bump_sequence(&mut self) {
        self.sequence += 1;
    }

    fn check_base(&self, args: &Value, required: bool) -> Result<(), (&'static str, String)> {
        match args.get("base_revision") {
            None if required => Err(("bad_request", "base_revision is required".to_string())),
            None => Ok(()),
            Some(value) => match value.as_u64() {
                Some(revision) if revision == self.revision => Ok(()),
                Some(revision) => Err((
                    "stale_revision",
                    format!("expected revision {}, got {}", self.revision, revision),
                )),
                None => Err(("bad_request", "base_revision must be an integer".to_string())),
            },
        }
    }

    fn create(&mut self, id: &str, args: &Value) -> Value {
        self.circuit_id = args
            .get("circuit_id")
            .and_then(Value::as_str)
            .unwrap_or("circ-1")
            .to_string();
        self.revision = 1;
        self.sequence = 0;
        self.circuit = Circuit::new();
        self.running = false;
        self.status = "ready".to_string();
        self.subscriptions.clear();
        self.loaded_dump = None;
        self.bump_sequence();
        ok(id, self.ack(id))
    }

    fn load_circuit(&mut self, id: &str, args: &Value) -> Value {
        if let Err((code, message)) = self.check_base(args, false) {
            return fail(id, code, message);
        }
        let dump = match args.get("dump").and_then(Value::as_str) {
            Some(dump) => dump.to_string(),
            None => return fail(id, "bad_request", "dump must be a string"),
        };
        let format = args.get("format").and_then(Value::as_str).unwrap_or("");
        if format != "text" && format != "xml" {
            return fail(id, "bad_request", "format must be 'text' or 'xml'");
        }
        self.loaded_dump = Some(dump);
        self.circuit.reset();
        self.running = false;
        self.status = "ready".to_string();
        self.revision += 1;
        self.bump_sequence();
        ok(id, self.ack(id))
    }

    fn topology_edit(&mut self, id: &str, args: &Value) -> Value {
        if let Err((code, message)) = self.check_base(args, true) {
            return fail(id, code, message);
        }
        let ops = match args.get("ops").and_then(Value::as_array) {
            Some(ops) if !ops.is_empty() => ops.to_vec(),
            _ => return fail(id, "bad_request", "ops must be a non-empty array"),
        };
        let mut operations = Vec::new();
        for op in &ops {
            let name = op.get("op").and_then(Value::as_str).unwrap_or("");
            let element_id = op.get("element_id").and_then(Value::as_str).unwrap_or("");
            if !matches!(name, "add" | "remove" | "connect" | "disconnect") || element_id.is_empty()
            {
                return fail(id, "bad_request", "invalid topology operation");
            }
            let node_ids: Vec<String> = op
                .get("node_ids")
                .and_then(Value::as_array)
                .map(|nodes| {
                    nodes
                        .iter()
                        .filter_map(|node| node.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let kind = op
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            operations.push(match name {
                "add" => Operation::Add {
                    id: element_id.to_string(),
                    kind,
                    nodes: node_ids,
                },
                "remove" => Operation::Remove {
                    id: element_id.to_string(),
                },
                "connect" => Operation::Connect {
                    id: element_id.to_string(),
                    nodes: node_ids,
                },
                _ => Operation::Disconnect {
                    id: element_id.to_string(),
                },
            });
        }
        if let Err(diagnostic) = self.circuit.apply_operations(operations) {
            return fail(id, "bad_request", format!("{diagnostic:?}"));
        }
        self.revision += 1;
        self.bump_sequence();
        ok(id, self.ack(id))
    }

    fn parameter_edit(&mut self, id: &str, args: &Value) -> Value {
        if let Err((code, message)) = self.check_base(args, true) {
            return fail(id, code, message);
        }
        let element_id = args.get("element_id").and_then(Value::as_str).unwrap_or("");
        let params = match args.get("params") {
            Some(Value::Object(entries)) => entries.clone(),
            _ => return fail(id, "bad_request", "params must be an object"),
        };
        if !self.circuit.elements.iter().any(|e| e.id == element_id) {
            return fail(id, "unknown_element", "element not found");
        }
        for (key, value) in params {
            let number = match value.as_f64() {
                Some(number) if number.is_finite() => number,
                _ => return fail(id, "bad_request", "parameter values must be finite numbers"),
            };
            if let Err(diagnostic) = self.circuit.set_parameter(element_id, &key, number) {
                return fail(id, "bad_request", format!("{diagnostic:?}"));
            }
        }
        self.revision += 1;
        self.bump_sequence();
        ok(id, self.ack(id))
    }

    fn reset(&mut self, id: &str, args: &Value) -> Value {
        if let Err((code, message)) = self.check_base(args, false) {
            return fail(id, code, message);
        }
        self.circuit.reset();
        self.running = false;
        self.status = "ready".to_string();
        self.bump_sequence();
        ok(id, self.ack(id))
    }

    fn set_running(&mut self, id: &str, args: &Value, running: bool) -> Value {
        if let Err((code, message)) = self.check_base(args, false) {
            return fail(id, code, message);
        }
        self.running = running;
        self.status = if running { "running" } else { "ready" }.to_string();
        self.bump_sequence();
        ok(id, self.ack(id))
    }

    fn step(&mut self, id: &str, args: &Value) -> Value {
        let dt = args.get("dt_s").and_then(Value::as_f64).unwrap_or(0.0);
        if !dt.is_finite() || dt <= 0.0 || dt > MAX_STEP_SECONDS {
            return fail(id, "bad_request", "dt_s must be in (0, 1]");
        }
        let steps = match args.get("steps") {
            None => 1,
            Some(value) => match value.as_u64() {
                Some(steps) => steps,
                None => return fail(id, "bad_request", "steps must be an integer"),
            },
        };
        if steps < 1 || steps > MAX_STEPS {
            return fail(id, "bad_request", "steps must be in [1, 10000]");
        }
        let outcome = self.circuit.step(dt, steps);
        if outcome.failed {
            self.status = "failed".to_string();
        }
        self.bump_sequence();
        ok(id, self.ack(id))
    }

    fn subscribe(&mut self, id: &str, args: &Value) -> Value {
        let stream_id = args.get("stream_id").and_then(Value::as_str).unwrap_or("");
        let signals = match args.get("signals").and_then(Value::as_array) {
            Some(signals) if !signals.is_empty() && signals.len() <= MAX_SUBSCRIBED_SIGNALS => {
                signals
            }
            _ => return fail(id, "bad_request", "signals must contain 1 to 64 entries"),
        };
        let mut parsed = Vec::new();
        for signal in signals {
            let signal_id = signal.get("id").and_then(Value::as_str).unwrap_or("");
            let element_id = signal.get("element_id").and_then(Value::as_str).unwrap_or("");
            let quantity = signal.get("quantity").and_then(Value::as_str).unwrap_or("");
            let unit = signal.get("unit").and_then(Value::as_str).unwrap_or("");
            if signal_id.is_empty()
                || !self.circuit.elements.iter().any(|e| e.id == element_id)
                || !matches!(quantity, "voltage" | "current")
                || !matches!(unit, "V" | "A")
            {
                return fail(id, "bad_request", "invalid measurement signal");
            }
            parsed.push(Signal {
                id: signal_id.to_string(),
                element_id: element_id.to_string(),
                quantity: quantity.to_string(),
                unit: unit.to_string(),
            });
        }
        self.subscriptions.retain(|s| s.stream_id != stream_id);
        self.subscriptions.push(Subscription {
            stream_id: stream_id.to_string(),
            signals: parsed,
        });
        self.bump_sequence();
        ok(id, self.ack(id))
    }

    fn unsubscribe(&mut self, id: &str, args: &Value) -> Value {
        let stream_id = args.get("stream_id").and_then(Value::as_str).unwrap_or("");
        let before = self.subscriptions.len();
        self.subscriptions.retain(|s| s.stream_id != stream_id);
        if self.subscriptions.len() == before {
            return fail(id, "unknown_stream", "stream not found");
        }
        self.bump_sequence();
        ok(id, self.ack(id))
    }

    fn snapshot(&self) -> Value {
        let nodes = Value::Array(
            self.circuit
                .node_voltages()
                .iter()
                .map(|(node, voltage)| {
                    obj(&[("id", text(node.clone())), ("voltage_v", num(*voltage))])
                })
                .collect(),
        );
        let mut measurements = Vec::new();
        for subscription in &self.subscriptions {
            for signal in &subscription.signals {
                let value = self
                    .circuit
                    .measurement(&signal.element_id, &signal.quantity)
                    .unwrap_or(0.0);
                measurements.push(obj(&[
                    ("id", text(signal.id.clone())),
                    ("element_id", text(signal.element_id.clone())),
                    ("quantity", text(signal.quantity.clone())),
                    ("value", num(value)),
                    ("unit", text(signal.unit.clone())),
                ]));
            }
        }
        obj(&[
            ("type", text("snapshot")),
            ("circuit_id", text(self.circuit_id.clone())),
            ("revision", num(self.revision as f64)),
            ("sequence", num(self.sequence as f64)),
            ("simulation_time_s", num(self.circuit.time_s())),
            ("time_step_s", num(self.circuit.current_dt())),
            ("running", Value::Bool(self.running)),
            ("status", text(self.status.clone())),
            ("nodes", nodes),
            ("measurements", Value::Array(measurements)),
            (
                "diagnostics",
                Value::Array(self.circuit.diagnostics.iter().map(|d| d.to_json()).collect()),
            ),
        ])
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, cmd: &str, args: &[(&str, Value)]) -> Value {
        obj(&[
            ("v", num(1.0)),
            ("id", text(id)),
            ("cmd", text(cmd)),
            ("args", obj(args)),
        ])
    }

    fn is_ok(response: &Value) -> bool {
        response.get("ok") == Some(&Value::Bool(true))
    }

    #[test]
    fn hello_needs_no_circuit() {
        let mut engine = Engine::new();
        let response = engine.handle(&request("c1", "hello", &[]));
        assert!(is_ok(&response));
        assert_eq!(
            response.get("result").and_then(|r| r.get("protocol")),
            Some(&num(1.0))
        );
    }

    #[test]
    fn rejects_wrong_protocol_version() {
        let mut request = request("c1", "hello", &[]);
        if let Value::Object(entries) = &mut request {
            entries[0].1 = num(2.0);
        }
        let response = Engine::new().handle(&request);
        assert_eq!(
            response.get("error").and_then(|e| e.get("code")),
            Some(&text("unsupported_version"))
        );
    }

    #[test]
    fn edits_require_current_revision() {
        let mut engine = Engine::new();
        engine.handle(&request("c1", "create", &[]));
        assert_eq!(engine.revision(), 1);
        let add = Value::Array(vec![obj(&[
            ("op", text("add")),
            ("element_id", text("r1")),
            ("kind", text("resistor")),
            ("node_ids", Value::Array(vec![text("n0"), text("n1")])),
        ])]);
        assert!(is_ok(&engine.handle(&request(
            "c2",
            "topology_edit",
            &[("base_revision", num(1.0)), ("ops", add)]
        ))));
        assert_eq!(engine.revision(), 2);
        let stale = engine.handle(&request(
            "c3",
            "parameter_edit",
            &[
                ("base_revision", num(1.0)),
                ("element_id", text("r1")),
                ("params", obj(&[("resistance_ohm", num(1000.0))])),
            ],
        ));
        assert_eq!(
            stale.get("error").and_then(|e| e.get("code")),
            Some(&text("stale_revision"))
        );
        let fresh = engine.handle(&request(
            "c4",
            "parameter_edit",
            &[
                ("base_revision", num(2.0)),
                ("element_id", text("r1")),
                ("params", obj(&[("resistance_ohm", num(1000.0))])),
            ],
        ));
        assert!(is_ok(&fresh));
        assert_eq!(engine.revision(), 3);
    }

    #[test]
    fn step_advances_time_and_sequence() {
        let mut engine = Engine::new();
        engine.handle(&request("c1", "create", &[]));
        let before = engine.sequence();
        let response = engine.handle(&request(
            "c2",
            "step",
            &[("dt_s", num(5e-6)), ("steps", num(100.0))],
        ));
        assert!(is_ok(&response));
        assert!((engine.simulation_time_s() - 5e-4).abs() < 1e-12);
        assert_eq!(engine.sequence(), before + 1);
    }

    #[test]
    fn snapshot_contains_subscribed_measurements() {
        let mut engine = Engine::new();
        engine.handle(&request("c1", "create", &[]));
        let add = Value::Array(vec![obj(&[
            ("op", text("add")),
            ("element_id", text("r1")),
            ("kind", text("resistor")),
            ("node_ids", Value::Array(vec![text("n0"), text("n1")])),
        ])]);
        engine.handle(&request(
            "c2",
            "topology_edit",
            &[("base_revision", num(1.0)), ("ops", add)],
        ));
        let signal = obj(&[
            ("id", text("vout")),
            ("element_id", text("r1")),
            ("quantity", text("voltage")),
            ("unit", text("V")),
        ]);
        assert!(is_ok(&engine.handle(&request(
            "c3",
            "subscribe",
            &[
                ("stream_id", text("scope-1")),
                ("signals", Value::Array(vec![signal])),
            ],
        ))));
        let snapshot = engine
            .handle(&request("c4", "snapshot", &[]))
            .get("result")
            .unwrap()
            .clone();
        assert_eq!(snapshot.get("revision"), Some(&num(2.0)));
        let measurements = snapshot.get("measurements").and_then(Value::as_array).unwrap();
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0].get("id"), Some(&text("vout")));
    }

    #[test]
    fn shutdown_returns_ack() {
        let mut engine = Engine::new();
        let response = engine.handle(&request("c1", "shutdown", &[]));
        assert!(is_ok(&response));
        assert_eq!(
            response.get("result").and_then(|r| r.get("type")),
            Some(&text("ack"))
        );
    }
}
