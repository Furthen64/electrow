//! Reproducible performance harness for the Electrow engine.
//!
//! Run with: cargo run --release --example perf_bench -- --out Temp/perf.json
//! It reports throughput, latency, iteration counts, simulated/wall ratio and
//! RSS for several circuit classes. No reference binary is required.

use std::fs;
use std::time::Instant;

use electrow_engine::json::Value;
use electrow_engine::solver::{Circuit, Operation};

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

fn text(value: &str) -> Value {
    Value::String(value.to_string())
}

fn add(circuit: &mut Circuit, id: &str, kind: &str, nodes: &[&str]) {
    circuit
        .apply_operations(vec![Operation::Add {
            id: id.to_string(),
            kind: kind.to_string(),
            nodes: nodes.iter().map(|node| (*node).to_string()).collect(),
        }])
        .unwrap();
}

fn param(circuit: &mut Circuit, id: &str, key: &str, value: f64) {
    circuit.set_parameter(id, key, value).unwrap();
}

fn divider() -> Circuit {
    let mut circuit = Circuit::new();
    add(&mut circuit, "gnd", "ground", &["gnd"]);
    add(&mut circuit, "v1", "voltage_source", &["n1", "gnd"]);
    add(&mut circuit, "r1", "resistor", &["n1", "n2"]);
    add(&mut circuit, "r2", "resistor", &["n2", "gnd"]);
    param(&mut circuit, "v1", "voltage_v", 10.0);
    param(&mut circuit, "r1", "resistance_ohm", 1000.0);
    param(&mut circuit, "r2", "resistance_ohm", 1000.0);
    circuit
}

fn rc() -> Circuit {
    let mut circuit = Circuit::new();
    add(&mut circuit, "gnd", "ground", &["gnd"]);
    add(&mut circuit, "v1", "voltage_source", &["n1", "gnd"]);
    add(&mut circuit, "r1", "resistor", &["n1", "n2"]);
    add(&mut circuit, "c1", "capacitor", &["n2", "gnd"]);
    param(&mut circuit, "v1", "voltage_v", 10.0);
    param(&mut circuit, "r1", "resistance_ohm", 1000.0);
    param(&mut circuit, "c1", "capacitance_f", 1e-6);
    circuit
}

fn ladder(length: usize) -> Circuit {
    let mut circuit = Circuit::new();
    add(&mut circuit, "gnd", "ground", &["gnd"]);
    add(&mut circuit, "v1", "voltage_source", &["n0", "gnd"]);
    param(&mut circuit, "v1", "voltage_v", 10.0);
    for index in 0..length {
        let id = format!("r{index}");
        let next = format!("n{}", index + 1);
        add(&mut circuit, &id, "resistor", &[&format!("n{index}"), &next]);
        param(&mut circuit, &id, "resistance_ohm", 1000.0);
    }
    let last = format!("n{length}");
    add(&mut circuit, "rlast", "resistor", &[&last, "gnd"]);
    param(&mut circuit, "rlast", "resistance_ohm", 1000.0);
    circuit
}

fn diode_chain(count: usize) -> Circuit {
    let mut circuit = Circuit::new();
    add(&mut circuit, "gnd", "ground", &["gnd"]);
    add(&mut circuit, "v1", "voltage_source", &["n0", "gnd"]);
    param(&mut circuit, "v1", "voltage_v", 10.0);
    for index in 0..count {
        let node = format!("n{}", index + 1);
        let resistor = format!("r{index}");
        add(&mut circuit, &resistor, "resistor", &[&format!("n{index}"), &node]);
        param(&mut circuit, &resistor, "resistance_ohm", 100.0);
        let diode = format!("d{index}");
        add(&mut circuit, &diode, "diode", &[&node, "gnd"]);
        param(&mut circuit, &diode, "is_a", 1e-12);
    }
    circuit
}

fn bjt_switch() -> Circuit {
    let mut circuit = Circuit::new();
    add(&mut circuit, "gnd", "ground", &["gnd"]);
    add(&mut circuit, "v1", "voltage_source", &["n1", "gnd"]);
    add(&mut circuit, "rb", "resistor", &["n1", "n2"]);
    add(&mut circuit, "rc", "resistor", &["n1", "n3"]);
    add(&mut circuit, "q1", "bjt", &["n2", "n3", "gnd"]);
    param(&mut circuit, "v1", "voltage_v", 10.0);
    param(&mut circuit, "rb", "resistance_ohm", 1e6);
    param(&mut circuit, "rc", "resistance_ohm", 1000.0);
    param(&mut circuit, "q1", "beta", 100.0);
    param(&mut circuit, "q1", "is_a", 1e-12);
    circuit
}

fn rss_kb() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = fs::read_to_string("/proc/self/statm") {
            if let Some(pages) = contents.split_whitespace().nth(1) {
                if let Ok(pages) = pages.parse::<f64>() {
                    return pages * 4.0;
                }
            }
        }
    }
    0.0
}

fn measure(circuit: &mut Circuit, dt: f64, steps: u64) -> (f64, f64, f64, u64, f64, f64) {
    let before_rss = rss_kb();
    let start = Instant::now();
    let outcome = circuit.step(dt, steps);
    let elapsed = start.elapsed().as_secs_f64();
    assert!(!outcome.failed, "benchmark step failed: {:?}", outcome.diagnostics);
    let advanced = outcome.advanced_s;
    (
        elapsed,
        advanced / elapsed.max(1e-12),
        steps as f64 / elapsed.max(1e-12),
        circuit.total_iterations,
        rss_kb() - before_rss,
        (elapsed * 1e6 / steps as f64),
    )
}

fn percentile(values: &[f64], fraction: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    sorted[index]
}

fn bench_case(name: &str, make: fn() -> Circuit, dt: f64, steps: u64, repeats: usize) -> Value {
    let mut runs = Vec::new();
    let mut latency = Vec::new();
    let mut circuit = make();
    for _ in 0..repeats {
        let (wall, sim_per_wall, steps_per_s, iterations, rss, per_step_us) =
            measure(&mut circuit, dt, steps);
        runs.push(obj(&[
            ("wall_s", num(wall)),
            ("sim_s_per_wall_s", num(sim_per_wall)),
            ("steps_per_s", num(steps_per_s)),
            ("iterations", num(iterations as f64)),
            ("rss_delta_kb", num(rss)),
        ]));
        latency.push(per_step_us);
        circuit.reset();
    }
    let walls: Vec<f64> = runs
        .iter()
        .map(|run| run.get("wall_s").and_then(Value::as_f64).unwrap())
        .collect();
    let mut profiled = make();
    profiled.perf.enabled = true;
    let profile_steps = steps.min(1000);
    let _ = measure(&mut profiled, dt, profile_steps);
    let measured = profiled.perf.stamp_ns + profiled.perf.solve_ns + profiled.perf.clone_ns;
    let pct = |value: u64| -> f64 {
        if measured == 0 { 0.0 } else { value as f64 * 100.0 / measured as f64 }
    };
    obj(&[
        ("name", text(name)),
        ("steps", num(steps as f64)),
        ("dt_s", num(dt)),
        ("runs", Value::Array(runs)),
        ("wall_median_s", num(percentile(&walls, 0.5))),
        ("step_us_p50", num(percentile(&latency, 0.5))),
        ("step_us_p95", num(percentile(&latency, 0.95))),
        ("elements", num(circuit.elements.len() as f64)),
        ("profile_steps", num(profile_steps as f64)),
        ("stamp_pct", num(pct(profiled.perf.stamp_ns))),
        ("solve_pct", num(pct(profiled.perf.solve_ns))),
        ("clone_pct", num(pct(profiled.perf.clone_ns))),
    ])
}

fn bench_topology_edits(repeats: usize) -> Value {
    let mut samples = Vec::new();
    for _ in 0..repeats {
        let mut circuit = Circuit::new();
        circuit.perf.enabled = true;
        add(&mut circuit, "gnd", "ground", &["gnd"]);
        for index in 0..60 {
            let id = format!("r{index}");
            add(&mut circuit, &id, "resistor", &[&format!("n{index}"), &format!("n{}", index + 1)]);
            param(&mut circuit, &id, "resistance_ohm", 1000.0);
        }
        let start = Instant::now();
        for index in 0..60 {
            let id = format!("r{index}");
            circuit
                .apply_operations(vec![Operation::Remove { id: id.clone() }])
                .unwrap();
            add(&mut circuit, &id, "resistor", &[&format!("n{index}"), &format!("n{}", index + 1)]);
            param(&mut circuit, &id, "resistance_ohm", 1000.0);
        }
        samples.push(start.elapsed().as_secs_f64());
    }
    let mut profiled = Circuit::new();
    profiled.perf.enabled = true;
    add(&mut profiled, "gnd", "ground", &["gnd"]);
    for index in 0..60 {
        let id = format!("r{index}");
        add(&mut profiled, &id, "resistor", &[&format!("n{index}"), &format!("n{}", index + 1)]);
        param(&mut profiled, &id, "resistance_ohm", 1000.0);
    }
    let analyze_before = profiled.perf.analyze_ns;
    for index in 0..10 {
        let id = format!("r{index}");
        profiled.apply_operations(vec![Operation::Remove { id: id.clone() }]).unwrap();
        add(&mut profiled, &id, "resistor", &[&format!("n{index}"), &format!("n{}", index + 1)]);
        param(&mut profiled, &id, "resistance_ohm", 1000.0);
    }
    obj(&[
        ("name", text("topology-edits")),
        ("edits", num(120.0)),
        ("runs", Value::Array(samples.iter().map(|value| num(*value)).collect())),
        ("wall_median_s", num(percentile(&samples, 0.5))),
        ("analyze_ms_per_20_edits", num((profiled.perf.analyze_ns - analyze_before) as f64 / 1e6)),
    ])
}

fn main() {
    let mut out = "Temp/perf.json".to_string();
    let args: Vec<String> = std::env::args().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--out") {
        if let Some(value) = args.get(index + 1) {
            out = value.clone();
        }
    }
    let repeats = 3;
    let cases = Value::Array(vec![
        bench_case("divider-small", divider, 5e-6, 10_000, repeats),
        bench_case("rc-small", rc, 5e-6, 10_000, repeats),
        bench_case("ladder-40", || ladder(40), 5e-6, 500, repeats),
        bench_case("ladder-120", || ladder(120), 5e-6, 100, repeats),
        bench_case("diodes-10", || diode_chain(10), 5e-6, 500, repeats),
        bench_case("bjt-switch", bjt_switch, 5e-6, 500, repeats),
        bench_topology_edits(repeats),
    ]);
    let report = obj(&[
        ("toolchain", text(env!("CARGO_PKG_VERSION"))),
        ("repeats", num(repeats as f64)),
        ("cases", cases),
    ]);
    if let Some(parent) = std::path::Path::new(&out).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&out, report.to_json()).unwrap();
    println!("performance report written to {out}");
}
