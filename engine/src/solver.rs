use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::json::Value;
use crate::models;

pub const DEFAULT_DT_S: f64 = 5e-6;
const MAX_STEP_DIAGNOSTICS: usize = 16;
const PIVOT_EPSILON: f64 = 1e-12;

#[derive(Clone, Debug, PartialEq)]
pub enum Diagnostic {
    NoGround { node_id: String },
    FloatingNode { node_id: String },
    SingularMatrix,
    InvalidValue { element_id: String, parameter: String },
    NonConvergence { iterations: u32, change_v: f64 },
}

fn obj(entries: &[(&str, Value)]) -> Value {
    Value::Object(
        entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect(),
    )
}

impl Diagnostic {
    pub fn to_json(&self) -> Value {
        match self {
            Diagnostic::NoGround { node_id } => obj(&[
                ("kind", Value::String("no_ground".to_string())),
                ("node_id", Value::String(node_id.clone())),
            ]),
            Diagnostic::FloatingNode { node_id } => obj(&[
                ("kind", Value::String("floating_node".to_string())),
                ("node_id", Value::String(node_id.clone())),
            ]),
            Diagnostic::SingularMatrix => {
                obj(&[("kind", Value::String("singular_matrix".to_string()))])
            }
            Diagnostic::InvalidValue {
                element_id,
                parameter,
            } => obj(&[
                ("kind", Value::String("invalid_value".to_string())),
                ("element_id", Value::String(element_id.clone())),
                ("parameter", Value::String(parameter.clone())),
            ]),
            Diagnostic::NonConvergence {
                iterations,
                change_v,
            } => obj(&[
                ("kind", Value::String("non_convergence".to_string())),
                ("iterations", Value::Number(*iterations as f64)),
                ("change_v", Value::Number(*change_v)),
            ]),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Element {
    pub id: String,
    pub kind: String,
    pub nodes: Vec<String>,
    pub params: HashMap<String, f64>,
}

impl Element {
    pub fn new(id: String, kind: String, nodes: Vec<String>) -> Self {
        let params = models::model_spec(&kind)
            .map(|spec| spec.default_params())
            .unwrap_or_default();
        Self {
            id,
            kind,
            nodes,
            params,
        }
    }

    pub fn validate(&self) -> Result<(), Diagnostic> {
        let spec = models::model_spec(&self.kind).ok_or_else(|| Diagnostic::InvalidValue {
            element_id: self.id.clone(),
            parameter: "kind".to_string(),
        })?;
        if self.nodes.len() != spec.terminals.len() {
            return Err(Diagnostic::InvalidValue {
                element_id: self.id.clone(),
                parameter: "nodes".to_string(),
            });
        }
        for param in spec.params {
            let value = self.params.get(param.key).copied().unwrap_or(param.default);
            if !value.is_finite() {
                return Err(Diagnostic::InvalidValue {
                    element_id: self.id.clone(),
                    parameter: param.key.to_string(),
                });
            }
            if param.binary && value != 0.0 && value != 1.0 {
                return Err(Diagnostic::InvalidValue {
                    element_id: self.id.clone(),
                    parameter: param.key.to_string(),
                });
            }
            if param.positive && value <= 0.0 {
                return Err(Diagnostic::InvalidValue {
                    element_id: self.id.clone(),
                    parameter: param.key.to_string(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Operation {
    Add {
        id: String,
        kind: String,
        nodes: Vec<String>,
    },
    Remove {
        id: String,
    },
    Connect {
        id: String,
        nodes: Vec<String>,
    },
    Disconnect {
        id: String,
    },
}

#[derive(Clone, Debug, Default)]
pub struct ElementState {
    pub v: f64,
    pub i: f64,
    pub v2: f64,
    pub v3: f64,
    pub g: f64,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub elements: HashMap<String, ElementState>,
    pub node_voltages: HashMap<String, f64>,
}

#[derive(Clone, Debug)]
pub struct SolverSettings {
    pub max_iterations: u32,
    pub convergence_tolerance_v: f64,
    pub max_diode_step_v: f64,
    pub min_dt_s: f64,
    pub max_dt_s: f64,
    pub max_halvings: u32,
    pub good_steps_before_growth: u32,
}

impl Default for SolverSettings {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            convergence_tolerance_v: 0.01,
            max_diode_step_v: 0.1,
            min_dt_s: 5e-11,
            max_dt_s: DEFAULT_DT_S,
            max_halvings: 8,
            good_steps_before_growth: 3,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StepOutcome {
    pub steps_done: u64,
    pub advanced_s: f64,
    pub failed: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Default)]
struct Matrix {
    a: Vec<Vec<f64>>,
    b: Vec<f64>,
    node_count: usize,
}

impl Matrix {
    fn reset(&mut self, size: usize, node_count: usize) {
        if self.a.len() != size {
            self.a = vec![vec![0.0; size]; size];
        } else {
            for row in &mut self.a {
                row.fill(0.0);
            }
        }
        if self.b.len() != size {
            self.b = vec![0.0; size];
        } else {
            self.b.fill(0.0);
        }
        self.node_count = node_count;
    }

    fn node_row(&self, node: usize) -> Option<usize> {
        if node == 0 {
            None
        } else {
            Some(node - 1)
        }
    }

    fn stamp_diag(&mut self, a: usize, g: f64) {
        if let Some(i) = self.node_row(a) {
            self.a[i][i] += g;
        }
    }

    fn stamp_offdiag(&mut self, a: usize, b: usize, g: f64) {
        if let (Some(i), Some(j)) = (self.node_row(a), self.node_row(b)) {
            self.a[i][j] += g;
            self.a[j][i] += g;
        }
    }

    fn stamp_rhs(&mut self, a: usize, r: f64) {
        if let Some(i) = self.node_row(a) {
            self.b[i] += r;
        }
    }

    fn stamp_conductance(&mut self, a: usize, b: usize, g: f64) {
        if let Some(i) = self.node_row(a) {
            self.a[i][i] += g;
        }
        if let Some(j) = self.node_row(b) {
            self.a[j][j] += g;
        }
        if let (Some(i), Some(j)) = (self.node_row(a), self.node_row(b)) {
            self.a[i][j] -= g;
            self.a[j][i] -= g;
        }
    }

    fn stamp_current(&mut self, a: usize, b: usize, current: f64) {
        if let Some(i) = self.node_row(a) {
            self.b[i] -= current;
        }
        if let Some(j) = self.node_row(b) {
            self.b[j] += current;
        }
    }

    fn stamp_voltage_source(&mut self, a: usize, b: usize, voltage: f64, index: usize) {
        let row = self.node_count - 1 + index;
        if let Some(i) = self.node_row(a) {
            self.a[i][row] += 1.0;
            self.a[row][i] += 1.0;
        }
        if let Some(j) = self.node_row(b) {
            self.a[j][row] -= 1.0;
            self.a[row][j] -= 1.0;
        }
        self.b[row] = voltage;
    }
}

#[derive(Clone)]
enum ElementMeta {
    Passive { g: f64, ieq: f64 },
    VoltageSource { index: usize },
    Vactrol { g_led: f64, ieq_led: f64, g_cell: f64 },
    None,
}

struct Assembly {
    meta: HashMap<String, ElementMeta>,
}

#[derive(Clone, Debug, Default)]
pub struct CircuitProfile {
    pub enabled: bool,
    pub stamp_ns: u64,
    pub solve_ns: u64,
    pub analyze_ns: u64,
    pub clone_ns: u64,
    pub steps: u64,
}

pub struct Circuit {
    pub elements: Vec<Element>,
    pub node_ids: Vec<String>,
    node_index: HashMap<String, usize>,
    voltage_sources: Vec<String>,
    state: State,
    pub time_s: f64,
    pub current_dt: f64,
    pub settings: SolverSettings,
    pub diagnostics: Vec<Diagnostic>,
    step_diagnostics: Vec<Diagnostic>,
    good_steps: u32,
    started: bool,
    pub total_iterations: u64,
    pub last_step_iterations: u32,
    pub perf: CircuitProfile,
    scratch: Matrix,
}

fn find_root(parent: &HashMap<String, String>, node: &str) -> String {
    let mut current = node.to_string();
    while let Some(next) = parent.get(&current) {
        if next == &current {
            break;
        }
        current = next.clone();
    }
    current
}

fn lu_solve_in_place(a: &mut [Vec<f64>], b: &mut [f64]) -> Result<Vec<f64>, Diagnostic> {
    let size = a.len();
    if size == 0 {
        return Ok(Vec::new());
    }
    if a.iter().flatten().any(|v| !v.is_finite()) || b.iter().any(|v| !v.is_finite()) {
        return Err(Diagnostic::InvalidValue {
            element_id: "matrix".to_string(),
            parameter: "value".to_string(),
        });
    }
    let column_scale: Vec<f64> = (0..size)
        .map(|col| {
            a.iter()
                .map(|row| row[col].abs())
                .fold(0.0_f64, f64::max)
        })
        .collect();
    for k in 0..size {
        let mut pivot = k;
        for row in k + 1..size {
            if a[row][k].abs() > a[pivot][k].abs() {
                pivot = row;
            }
        }
        if a[pivot][k].abs() <= PIVOT_EPSILON * column_scale[k] {
            return Err(Diagnostic::SingularMatrix);
        }
        if pivot != k {
            a.swap(k, pivot);
            b.swap(k, pivot);
        }
        for row in k + 1..size {
            let factor = a[row][k] / a[k][k];
            a[row][k] = factor;
            for col in k + 1..size {
                a[row][col] -= factor * a[k][col];
            }
            b[row] -= factor * b[k];
        }
    }
    let mut x = vec![0.0; size];
    for row in (0..size).rev() {
        let mut sum = b[row];
        for col in row + 1..size {
            sum -= a[row][col] * x[col];
        }
        x[row] = sum / a[row][row];
    }
    if x.iter().any(|v| !v.is_finite()) {
        return Err(Diagnostic::InvalidValue {
            element_id: "matrix".to_string(),
            parameter: "solution".to_string(),
        });
    }
    Ok(x)
}

impl Circuit {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            node_ids: Vec::new(),
            node_index: HashMap::new(),
            voltage_sources: Vec::new(),
            state: State::default(),
            time_s: 0.0,
            current_dt: DEFAULT_DT_S,
            settings: SolverSettings::default(),
            diagnostics: Vec::new(),
            step_diagnostics: Vec::new(),
            good_steps: 0,
            started: false,
            total_iterations: 0,
            last_step_iterations: 0,
            perf: CircuitProfile::default(),
            scratch: Matrix::default(),
        }
    }

    pub fn time_s(&self) -> f64 {
        self.time_s
    }

    pub fn current_dt(&self) -> f64 {
        self.current_dt
    }

    pub fn all_diagnostics(&self) -> Vec<Diagnostic> {
        let mut all = self.diagnostics.clone();
        for diagnostic in &self.step_diagnostics {
            if !all.contains(diagnostic) {
                all.push(diagnostic.clone());
            }
        }
        all
    }

    fn record_step_diagnostic(&mut self, diagnostic: Diagnostic) {
        if self.step_diagnostics.len() < MAX_STEP_DIAGNOSTICS
            && !self.step_diagnostics.contains(&diagnostic)
        {
            self.step_diagnostics.push(diagnostic);
        }
    }

    pub fn node_voltages(&self) -> Vec<(String, f64)> {
        self.node_ids
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    *self.state.node_voltages.get(id).unwrap_or(&0.0),
                )
            })
            .collect()
    }

    pub fn measurement(&self, element_id: &str, quantity: &str) -> Option<f64> {
        let element = self.elements.iter().find(|e| e.id == element_id)?;
        if element.nodes.len() < 2 {
            return None;
        }
        let a = self
            .state
            .node_voltages
            .get(&element.nodes[0])
            .copied()
            .unwrap_or(0.0);
        let b = self
            .state
            .node_voltages
            .get(&element.nodes[1])
            .copied()
            .unwrap_or(0.0);
        let voltage = a - b;
        if quantity == "voltage" {
            return match element.kind.as_str() {
                "bjt" => {
                    let collector = self.state.node_voltages.get(&element.nodes[1]).copied().unwrap_or(0.0);
                    let emitter = self.state.node_voltages.get(&element.nodes[2]).copied().unwrap_or(0.0);
                    Some(collector - emitter)
                }
                "mosfet" => {
                    let source = self.state.node_voltages.get(&element.nodes[1]).copied().unwrap_or(0.0);
                    let drain = self.state.node_voltages.get(&element.nodes[2]).copied().unwrap_or(0.0);
                    Some(drain - source)
                }
                _ => Some(voltage),
            };
        }
        if quantity != "current" {
            return None;
        }
        match element.kind.as_str() {
            "resistor" => Some(voltage / element.params.get("resistance_ohm").copied()?),
            "current_source" => Some(element.params.get("current_a").copied()?),
            "wire" => Some(0.0),
            "switch" => {
                let closed = element.params.get("closed").copied().unwrap_or(1.0);
                let resistance = element.params.get("resistance_ohm").copied().unwrap_or(1e-3);
                if closed == 1.0 {
                    Some(voltage / resistance)
                } else {
                    Some(0.0)
                }
            }
            _ => Some(self.state.elements.get(element_id).map(|s| s.i).unwrap_or(0.0)),
        }
    }

    pub fn apply_operations(&mut self, ops: Vec<Operation>) -> Result<(), Diagnostic> {
        for op in ops {
            match op {
                Operation::Add { id, kind, nodes } => {
                    if id.is_empty() || nodes.is_empty() {
                        return Err(Diagnostic::InvalidValue {
                            element_id: id,
                            parameter: "nodes".to_string(),
                        });
                    }
                    if self.elements.iter().any(|e| e.id == id) {
                        return Err(Diagnostic::InvalidValue {
                            element_id: id,
                            parameter: "id".to_string(),
                        });
                    }
                    let element = Element::new(id, kind, nodes);
                    element.validate()?;
                    self.elements.push(element);
                }
                Operation::Remove { id } => {
                    if !self.elements.iter().any(|e| e.id == id) {
                        return Err(Diagnostic::InvalidValue {
                            element_id: id,
                            parameter: "id".to_string(),
                        });
                    }
                    self.elements.retain(|e| e.id != id);
                    self.state.elements.remove(&id);
                }
                Operation::Connect { id, nodes } => {
                    let element = self.elements.iter_mut().find(|e| e.id == id);
                    match element {
                        Some(element) => {
                            if nodes.is_empty() {
                                return Err(Diagnostic::InvalidValue {
                                    element_id: id,
                                    parameter: "nodes".to_string(),
                                });
                            }
                            element.nodes = nodes;
                            element.validate()?;
                        }
                        None => {
                            return Err(Diagnostic::InvalidValue {
                                element_id: id,
                                parameter: "id".to_string(),
                            })
                        }
                    }
                }
                Operation::Disconnect { id } => {
                    let element = self.elements.iter_mut().find(|e| e.id == id);
                    match element {
                        Some(element) => element.nodes.clear(),
                        None => {
                            return Err(Diagnostic::InvalidValue {
                                element_id: id,
                                parameter: "id".to_string(),
                            })
                        }
                    }
                }
            }
        }
        self.analyze();
        Ok(())
    }

    pub fn set_parameter(&mut self, element_id: &str, key: &str, value: f64) -> Result<(), Diagnostic> {
        let element = self.elements.iter_mut().find(|e| e.id == element_id);
        let element = match element {
            Some(element) => element,
            None => {
                return Err(Diagnostic::InvalidValue {
                    element_id: element_id.to_string(),
                    parameter: "id".to_string(),
                })
            }
        };
        element.params.insert(key.to_string(), value);
        element.validate()?;
        self.refresh_diagnostics();
        Ok(())
    }

    pub fn analyze(&mut self) {
        let profile_start = if self.perf.enabled { Some(Instant::now()) } else { None };
        let mut parent: HashMap<String, String> = HashMap::new();
        for element in &self.elements {
            for node in &element.nodes {
                parent.entry(node.clone()).or_insert_with(|| node.clone());
            }
        }
        for element in &self.elements {
            if element.kind == "wire" && element.nodes.len() == 2 {
                let a = find_root(&parent, &element.nodes[0]);
                let b = find_root(&parent, &element.nodes[1]);
                if a != b {
                    parent.insert(b, a);
                }
            }
        }
        for element in &mut self.elements {
            for node in &mut element.nodes {
                *node = find_root(&parent, node);
            }
        }

        let mut nodes = Vec::new();
        for element in &self.elements {
            for node in &element.nodes {
                if !nodes.contains(node) {
                    nodes.push(node.clone());
                }
            }
        }
        let ground = self
            .elements
            .iter()
            .find(|e| e.kind == "ground")
            .and_then(|e| e.nodes.first().cloned())
            .or_else(|| nodes.iter().find(|node| node.as_str() == "gnd").cloned())
            .or_else(|| nodes.first().cloned());

        let mut ordered = Vec::new();
        if let Some(node) = &ground {
            ordered.push(node.clone());
        }
        for node in &nodes {
            if Some(node) != ground.as_ref() {
                ordered.push(node.clone());
            }
        }
        self.node_ids = ordered;
        self.node_index = self
            .node_ids
            .iter()
            .enumerate()
            .map(|(index, node)| (node.clone(), index))
            .collect();
        self.voltage_sources = self
            .elements
            .iter()
            .filter(|e| e.kind == "voltage_source" && e.nodes.len() == 2)
            .map(|e| e.id.clone())
            .collect();

        self.refresh_diagnostics();
        if let Some(start) = profile_start {
            self.perf.analyze_ns += start.elapsed().as_nanos() as u64;
        }
    }

    fn refresh_diagnostics(&mut self) {
        self.diagnostics.clear();
        for element in &self.elements {
            if let Err(diagnostic) = element.validate() {
                self.diagnostics.push(diagnostic);
            }
        }
        if !self.node_ids.is_empty()
            && !self.elements.iter().any(|e| e.kind == "ground")
            && !self.node_ids.iter().any(|node| node == "gnd")
        {
            self.diagnostics.push(Diagnostic::NoGround {
                node_id: self.node_ids[0].clone(),
            });
        }

        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for element in &self.elements {
            for (i, a) in element.nodes.iter().enumerate() {
                for b in element.nodes.iter().skip(i + 1) {
                    adjacency.entry(a.clone()).or_default().push(b.clone());
                    adjacency.entry(b.clone()).or_default().push(a.clone());
                }
            }
        }
        if let Some(reference) = self.node_ids.first() {
            let mut visited = HashSet::new();
            let mut stack = vec![reference.clone()];
            while let Some(node) = stack.pop() {
                if visited.insert(node.clone()) {
                    if let Some(neighbors) = adjacency.get(&node) {
                        stack.extend(neighbors.iter().cloned());
                    }
                }
            }
            for node in &self.node_ids {
                if !visited.contains(node) {
                    self.diagnostics.push(Diagnostic::FloatingNode {
                        node_id: node.clone(),
                    });
                }
            }
        }
    }


    pub fn reset(&mut self) {
        self.state = State::default();
        self.time_s = 0.0;
        self.current_dt = DEFAULT_DT_S;
        self.good_steps = 0;
        self.started = false;
        self.last_step_iterations = 0;
        self.step_diagnostics.clear();
        self.analyze();
    }

    fn mosfet_calc(
        vs: [f64; 3],
        pnp: f64,
        vt: f64,
        beta: f64,
    ) -> (usize, usize, f64, f64, f64, f64) {
        let pn = if pnp == 0.0 { 1.0 } else { -1.0 };
        let mut source = 1usize;
        let mut drain = 2usize;
        if pn * vs[1] > pn * vs[2] {
            source = 2;
            drain = 1;
        }
        let realvgs = vs[0] - vs[source];
        let realvds = vs[drain] - vs[source];
        let vgs = realvgs * pn;
        let vds = realvds * pn;
        let (mut ids, gm, gds) = if vgs < vt {
            let gds = 1e-8;
            (vds * gds, 0.0, gds)
        } else if vds < vgs - vt {
            let gm = beta * vds;
            let gds = beta * (vgs - vds - vt);
            (beta * ((vgs - vt) * vds - vds * vds * 0.5), gm, gds)
        } else {
            let gm = beta * (vgs - vt);
            let gds = 1e-8;
            (0.5 * beta * (vgs - vt) * (vgs - vt) + (vds - (vgs - vt)) * gds, gm, gds)
        };
        let ids0 = ids;
        if (source == 2 && pn == 1.0) || (source == 1 && pn == -1.0) {
            ids = -ids;
        }
        let rs = -pn * ids0 + gds * realvds + gm * realvgs;
        (source, drain, ids, gm, gds, rs)
    }

    fn diode_parameters(element: &Element) -> (f64, f64) {
        let is = element.params.get("is_a").copied().unwrap_or(1e-12);
        let emission = element.params.get("emission").copied().unwrap_or(1.0);
        (is, emission * 0.025865)
    }

    fn diode_current(element: &Element, voltage: f64) -> (f64, f64) {
        let (is, thermal) = Self::diode_parameters(element);
        let exponent = (voltage / thermal).clamp(-100.0, 50.0);
        let exp = exponent.exp();
        let current = is * (exp - 1.0);
        let conductance = is * exp / thermal;
        (current, conductance)
    }

    fn vactrol_led_current(voltage: f64) -> (f64, f64) {
        const IS: f64 = 3.203535273232067e-9;
        const EMISSION: f64 = 4.077;
        let thermal = EMISSION * 0.025865;
        let exponent = (voltage / thermal).clamp(-100.0, 50.0);
        let exp = exponent.exp();
        (IS * (exp - 1.0), IS * exp / thermal)
    }

    fn vactrol_target_conductance(element: &Element, led_current: f64) -> f64 {
        let dark = element.params.get("dark_resistance_ohm").copied().unwrap_or(10e6);
        let at_one_ma = element.params.get("resistance_at_1ma_ohm").copied().unwrap_or(30e3);
        let exponent = element.params.get("response_exponent").copied().unwrap_or(0.8);
        let minimum = element.params.get("minimum_resistance_ohm").copied().unwrap_or(1.5e3);
        if !(led_current > 0.0) {
            return 1.0 / dark;
        }
        let resistance = at_one_ma * (led_current * 1000.0).powf(-exponent);
        let resistance = if !resistance.is_finite() || resistance > dark {
            dark
        } else if resistance < minimum {
            minimum
        } else {
            resistance
        };
        1.0 / resistance
    }

    fn vactrol_advance_conductance(
        element: &Element,
        conductance: f64,
        target: f64,
        dt: f64,
    ) -> f64 {
        let turn_on = element.params.get("turn_on_s").copied().unwrap_or(2.5e-3);
        let turn_off = element.params.get("turn_off_s").copied().unwrap_or(8.403422185645153e-3);
        let time_constant = if target > conductance { turn_on } else { turn_off };
        if !(time_constant > 0.0) || !(dt > 0.0) {
            return target;
        }
        conductance + (target - conductance) * (1.0 - (-dt / time_constant).exp())
    }

    fn assemble(&self, matrix: &mut Matrix, state: &State, dt: f64) -> Result<Assembly, Diagnostic> {
        let node_count = self.node_ids.len();
        let size = node_count.saturating_sub(1) + self.voltage_sources.len();
        matrix.reset(size, node_count);
        let mut meta = HashMap::new();

        for element in &self.elements {
            if element.nodes.len() < 2 {
                meta.insert(element.id.clone(), ElementMeta::None);
                continue;
            }
            let idxs: Vec<usize> = element
                .nodes
                .iter()
                .map(|node| {
                    self.node_index.get(node).copied().ok_or_else(|| Diagnostic::InvalidValue {
                        element_id: element.id.clone(),
                        parameter: "nodes".to_string(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let a = idxs[0];
            let b = idxs[1];
            match element.kind.as_str() {
                "resistor" => {
                    let resistance = element.params.get("resistance_ohm").copied().unwrap_or(0.0);
                    if resistance <= 0.0 || !resistance.is_finite() {
                        return Err(Diagnostic::InvalidValue {
                            element_id: element.id.clone(),
                            parameter: "resistance_ohm".to_string(),
                        });
                    }
                    matrix.stamp_conductance(a, b, 1.0 / resistance);
                    meta.insert(element.id.clone(), ElementMeta::None);
                }
                "voltage_source" => {
                    let voltage = element.params.get("voltage_v").copied().unwrap_or(0.0);
                    let index = self
                        .voltage_sources
                        .iter()
                        .position(|id| id == &element.id)
                        .unwrap_or(0);
                    matrix.stamp_voltage_source(a, b, voltage, index);
                    meta.insert(element.id.clone(), ElementMeta::VoltageSource { index });
                }
                "current_source" => {
                    let current = element.params.get("current_a").copied().unwrap_or(0.0);
                    matrix.stamp_current(a, b, current);
                    meta.insert(element.id.clone(), ElementMeta::None);
                }
                "capacitor" => {
                    let capacitance = element.params.get("capacitance_f").copied().unwrap_or(0.0);
                    if capacitance <= 0.0 || !capacitance.is_finite() {
                        return Err(Diagnostic::InvalidValue {
                            element_id: element.id.clone(),
                            parameter: "capacitance_f".to_string(),
                        });
                    }
                    let previous = state.elements.get(&element.id).cloned().unwrap_or_default();
                    let g = 2.0 * capacitance / dt;
                    let ieq = -g * previous.v - previous.i;
                    matrix.stamp_conductance(a, b, g);
                    matrix.stamp_current(a, b, ieq);
                    meta.insert(element.id.clone(), ElementMeta::Passive { g, ieq });
                }
                "inductor" => {
                    let inductance = element.params.get("inductance_h").copied().unwrap_or(0.0);
                    if inductance <= 0.0 || !inductance.is_finite() {
                        return Err(Diagnostic::InvalidValue {
                            element_id: element.id.clone(),
                            parameter: "inductance_h".to_string(),
                        });
                    }
                    let previous = state.elements.get(&element.id).cloned().unwrap_or_default();
                    let g = dt / (2.0 * inductance);
                    let ieq = previous.i + g * previous.v;
                    matrix.stamp_conductance(a, b, g);
                    matrix.stamp_current(a, b, ieq);
                    meta.insert(element.id.clone(), ElementMeta::Passive { g, ieq });
                }
                "diode" => {
                    let previous = state.elements.get(&element.id).cloned().unwrap_or_default();
                    let (current, g) = Self::diode_current(element, previous.v);
                    let ieq = current - g * previous.v;
                    matrix.stamp_conductance(a, b, g);
                    matrix.stamp_current(a, b, ieq);
                    meta.insert(element.id.clone(), ElementMeta::Passive { g, ieq });
                }
                "vactrol" => {
                    let previous = state.elements.get(&element.id).cloned().unwrap_or_default();
                    let (led_current, g_led) = Self::vactrol_led_current(previous.v);
                    let ieq_led = led_current - g_led * previous.v;
                    matrix.stamp_conductance(a, b, g_led);
                    matrix.stamp_current(a, b, ieq_led);
                    let dark = element.params.get("dark_resistance_ohm").copied().unwrap_or(10e6);
                    let g_cell = if previous.g.is_finite() && previous.g > 0.0 {
                        previous.g
                    } else {
                        1.0 / dark
                    };
                    matrix.stamp_conductance(idxs[2], idxs[3], g_cell);
                    meta.insert(
                        element.id.clone(),
                        ElementMeta::Vactrol {
                            g_led,
                            ieq_led,
                            g_cell,
                        },
                    );
                }
                "switch" => {
                    let closed = element.params.get("closed").copied().unwrap_or(1.0);
                    let resistance = element.params.get("resistance_ohm").copied().unwrap_or(1e-3);
                    if closed == 1.0 {
                        if resistance <= 0.0 || !resistance.is_finite() {
                            return Err(Diagnostic::InvalidValue {
                                element_id: element.id.clone(),
                                parameter: "resistance_ohm".to_string(),
                            });
                        }
                        matrix.stamp_conductance(a, b, 1.0 / resistance);
                    }
                    meta.insert(element.id.clone(), ElementMeta::None);
                }
                "bjt" => {
                    let previous = state.elements.get(&element.id).cloned().unwrap_or_default();
                    let beta = element.params.get("beta").copied().unwrap_or(100.0);
                    let pnp = element.params.get("pnp").copied().unwrap_or(0.0);
                    let is = element.params.get("is_a").copied().unwrap_or(1e-12);
                    let emission = element.params.get("emission").copied().unwrap_or(1.0);
                    let thermal = emission * 0.025865;
                    let exponent = (previous.v / thermal).clamp(-100.0, 50.0);
                    let exp = exponent.exp();
                    let current = is * (exp - 1.0);
                    let g = is * exp / thermal;
                    let ieq = current - g * previous.v;
                    let collector = idxs[1];
                    let emitter = idxs[2];
                    matrix.stamp_conductance(a, emitter, g);
                    if pnp == 1.0 {
                        matrix.stamp_current(emitter, a, ieq);
                        matrix.stamp_current(emitter, collector, beta * current);
                    } else {
                        matrix.stamp_current(a, emitter, ieq);
                        matrix.stamp_current(collector, emitter, beta * current);
                    }
                    meta.insert(element.id.clone(), ElementMeta::None);
                }
                "mosfet" => {
                    let previous = state.elements.get(&element.id).cloned().unwrap_or_default();
                    let pnp = element.params.get("pnp").copied().unwrap_or(0.0);
                    let vt = element.params.get("vt").copied().unwrap_or(1.5);
                    let beta = element.params.get("beta").copied().unwrap_or(0.02);
                    let vs = [previous.v, previous.v2, previous.v3];
                    let (source, drain, _ids, gm, gds, rs) =
                        Self::mosfet_calc(vs, pnp, vt, beta);
                    let gate = idxs[0];
                    let source_node = idxs[source];
                    let drain_node = idxs[drain];
                    matrix.stamp_diag(drain_node, gds);
                    matrix.stamp_offdiag(drain_node, source_node, -gds - gm);
                    matrix.stamp_offdiag(drain_node, gate, gm);
                    matrix.stamp_diag(source_node, gds + gm);
                    matrix.stamp_offdiag(source_node, drain_node, -gds);
                    matrix.stamp_offdiag(source_node, gate, -gm);
                    matrix.stamp_rhs(drain_node, rs);
                    matrix.stamp_rhs(source_node, -rs);
                    meta.insert(element.id.clone(), ElementMeta::None);
                }
                _ => {
                    meta.insert(element.id.clone(), ElementMeta::None);
                }
            }
        }
        if matrix.a.iter().flatten().any(|v| !v.is_finite())
            || matrix.b.iter().any(|v| !v.is_finite())
        {
            return Err(Diagnostic::InvalidValue {
                element_id: "matrix".to_string(),
                parameter: "value".to_string(),
            });
        }
        Ok(Assembly { meta })
    }

    fn extract_voltages(&self, solution: &[f64]) -> Vec<f64> {
        let mut voltages = vec![0.0; self.node_ids.len()];
        for index in 1..self.node_ids.len() {
            voltages[index] = solution[index - 1];
        }
        voltages
    }

    fn commit(
        &mut self,
        mut state: State,
        solution: &[f64],
        voltages: &[f64],
        assembly: &Assembly,
        dt: f64,
    ) -> Result<(), Diagnostic> {
        for (index, node) in self.node_ids.iter().enumerate() {
            state
                .node_voltages
                .insert(node.clone(), voltages.get(index).copied().unwrap_or(0.0));
        }
        let unknown_offset = self.node_ids.len().saturating_sub(1);
        for element in &self.elements {
            if element.nodes.len() < 2 {
                continue;
            }
            let a = self.node_index.get(&element.nodes[0]).copied().unwrap_or(0);
            let b = self.node_index.get(&element.nodes[1]).copied().unwrap_or(0);
            let voltage = voltages.get(a).copied().unwrap_or(0.0)
                - voltages.get(b).copied().unwrap_or(0.0);
            let updated_g = state.elements.get(&element.id).map(|state| state.g).unwrap_or(0.0);
            let entry = state.elements.entry(element.id.clone()).or_default();
            match assembly.meta.get(&element.id) {
                Some(ElementMeta::VoltageSource { index }) => {
                    entry.i = solution.get(unknown_offset + index).copied().unwrap_or(0.0);
                    entry.v = voltage;
                }
                Some(ElementMeta::Passive { g, ieq }) => {
                    entry.i = g * voltage + ieq;
                    entry.v = voltage;
                }
                Some(ElementMeta::Vactrol {
                    g_led,
                    ieq_led,
                    g_cell,
                }) => {
                    let cell_a = self.node_index.get(&element.nodes[2]).copied().unwrap_or(0);
                    let cell_b = self.node_index.get(&element.nodes[3]).copied().unwrap_or(0);
                    let cell_voltage = voltages.get(cell_a).copied().unwrap_or(0.0)
                        - voltages.get(cell_b).copied().unwrap_or(0.0);
                    let conductance = if updated_g.is_finite() && updated_g > 0.0 {
                        updated_g
                    } else {
                        *g_cell
                    };
                    entry.i = conductance * cell_voltage;
                    entry.v2 = g_led * voltage + ieq_led;
                    entry.v = voltage;
                    entry.g = conductance;
                }
                _ => match element.kind.as_str() {
                    "bjt" => {
                        let pnp = element.params.get("pnp").copied().unwrap_or(0.0);
                        let beta = element.params.get("beta").copied().unwrap_or(100.0);
                        let is = element.params.get("is_a").copied().unwrap_or(1e-12);
                        let emission = element.params.get("emission").copied().unwrap_or(1.0);
                        let base = voltages
                            .get(self.node_index.get(&element.nodes[0]).copied().unwrap_or(0))
                            .copied()
                            .unwrap_or(0.0);
                        let emitter = voltages
                            .get(self.node_index.get(&element.nodes[2]).copied().unwrap_or(0))
                            .copied()
                            .unwrap_or(0.0);
                        let pn = if pnp == 0.0 { 1.0 } else { -1.0 };
                        let vj = pn * (base - emitter);
                        let thermal = emission * 0.025865;
                        let exponent = (vj / thermal).clamp(-100.0, 50.0);
                        let current = is * (exponent.exp() - 1.0);
                        entry.i = beta * current;
                        entry.v = vj;
                    }
                    "mosfet" => {
                        let pnp = element.params.get("pnp").copied().unwrap_or(0.0);
                        let vt = element.params.get("vt").copied().unwrap_or(1.5);
                        let beta = element.params.get("beta").copied().unwrap_or(0.02);
                        let vs: [f64; 3] = [
                            voltages
                                .get(self.node_index.get(&element.nodes[0]).copied().unwrap_or(0))
                                .copied()
                                .unwrap_or(0.0),
                            voltages
                                .get(self.node_index.get(&element.nodes[1]).copied().unwrap_or(0))
                                .copied()
                                .unwrap_or(0.0),
                            voltages
                                .get(self.node_index.get(&element.nodes[2]).copied().unwrap_or(0))
                                .copied()
                                .unwrap_or(0.0),
                        ];
                        let (_source, _drain, ids, _gm, _gds, _rs) =
                            Self::mosfet_calc(vs, pnp, vt, beta);
                        entry.i = ids;
                        entry.v = vs[0];
                    }
                    _ => {
                        entry.v = voltage;
                    }
                },
            }
        }
        self.state = state;
        self.time_s += dt;
        Ok(())
    }

    fn attempt_step(&mut self, dt: f64) -> Result<(), Diagnostic> {
        let clone_start = if self.perf.enabled { Some(Instant::now()) } else { None };
        let mut working = self.state.clone();
        if let Some(start) = clone_start {
            self.perf.clone_ns += start.elapsed().as_nanos() as u64;
        }
        let nonlinear_indices: Vec<usize> = self
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.kind.as_str(), "diode" | "bjt" | "mosfet" | "vactrol"))
            .map(|(index, _)| index)
            .collect();
        let mut iterations = 0;
        let mut matrix = std::mem::take(&mut self.scratch);
        loop {
            let stamp_start = if self.perf.enabled { Some(Instant::now()) } else { None };
            let assembly = self.assemble(&mut matrix, &working, dt)?;
            if let Some(start) = stamp_start {
                self.perf.stamp_ns += start.elapsed().as_nanos() as u64;
            }
            let solve_start = if self.perf.enabled { Some(Instant::now()) } else { None };
            let solution = lu_solve_in_place(&mut matrix.a, &mut matrix.b)?;
            if let Some(start) = solve_start {
                self.perf.solve_ns += start.elapsed().as_nanos() as u64;
            }
            let voltages = self.extract_voltages(&solution);
            let mut max_change = 0.0;
            for &index in &nonlinear_indices {
                let element = &self.elements[index];
                let id = &element.id;
                match element.kind.as_str() {
                    "diode" => {
                        let a = self.node_index.get(&element.nodes[0]).copied().unwrap_or(0);
                        let b = self.node_index.get(&element.nodes[1]).copied().unwrap_or(0);
                        let new_voltage = voltages.get(a).copied().unwrap_or(0.0)
                            - voltages.get(b).copied().unwrap_or(0.0);
                        let previous = working.elements.entry(id.clone()).or_default();
                        let change = (new_voltage - previous.v).abs();
                        if change > max_change {
                            max_change = change;
                        }
                        let limit = self.settings.max_diode_step_v;
                        previous.v = if change > limit {
                            previous.v + (new_voltage - previous.v).signum() * limit
                        } else {
                            new_voltage
                        };
                    }
                    "vactrol" => {
                        let anode = self.node_index.get(&element.nodes[0]).copied().unwrap_or(0);
                        let cathode = self.node_index.get(&element.nodes[1]).copied().unwrap_or(0);
                        let new_led_voltage = voltages.get(anode).copied().unwrap_or(0.0)
                            - voltages.get(cathode).copied().unwrap_or(0.0);
                        let previous = working.elements.entry(id.clone()).or_default();
                        let change = (new_led_voltage - previous.v).abs();
                        if change > max_change {
                            max_change = change;
                        }
                        let limit = self.settings.max_diode_step_v;
                        previous.v = if change > limit {
                            previous.v + (new_led_voltage - previous.v).signum() * limit
                        } else {
                            new_led_voltage
                        };
                        let led_current = Self::vactrol_led_current(previous.v).0.max(0.0);
                        let target = Self::vactrol_target_conductance(element, led_current);
                        if previous.g <= 0.0 || !previous.g.is_finite() {
                            previous.g = 1.0 / element
                                .params
                                .get("dark_resistance_ohm")
                                .copied()
                                .unwrap_or(10e6);
                        }
                        previous.g =
                            Self::vactrol_advance_conductance(element, previous.g, target, dt);
                    }
                    "bjt" => {
                        let pnp = element.params.get("pnp").copied().unwrap_or(0.0);
                        let pn = if pnp == 0.0 { 1.0 } else { -1.0 };
                        let base = voltages
                            .get(self.node_index.get(&element.nodes[0]).copied().unwrap_or(0))
                            .copied()
                            .unwrap_or(0.0);
                        let emitter = voltages
                            .get(self.node_index.get(&element.nodes[2]).copied().unwrap_or(0))
                            .copied()
                            .unwrap_or(0.0);
                        let new_vj = pn * (base - emitter);
                        let previous = working.elements.entry(id.clone()).or_default();
                        let change = (new_vj - previous.v).abs();
                        if change > max_change {
                            max_change = change;
                        }
                        let limit = self.settings.max_diode_step_v;
                        previous.v = if change > limit {
                            previous.v + (new_vj - previous.v).signum() * limit
                        } else {
                            new_vj
                        };
                    }
                    "mosfet" => {
                        let beta = element.params.get("beta").copied().unwrap_or(0.02);
                        let vs = [
                            voltages
                                .get(self.node_index.get(&element.nodes[0]).copied().unwrap_or(0))
                                .copied()
                                .unwrap_or(0.0),
                            voltages
                                .get(self.node_index.get(&element.nodes[1]).copied().unwrap_or(0))
                                .copied()
                                .unwrap_or(0.0),
                            voltages
                                .get(self.node_index.get(&element.nodes[2]).copied().unwrap_or(0))
                                .copied()
                                .unwrap_or(0.0),
                        ];
                        let previous = working.elements.entry(id.clone()).or_default();
                        let change = (vs[0] - previous.v)
                            .abs()
                            .max((vs[1] - previous.v2).abs())
                            .max((vs[2] - previous.v3).abs());
                        let change_scaled = if beta > 1.0 { change * 100.0 } else { change };
                        if change_scaled > max_change {
                            max_change = change_scaled;
                        }
                        const LIMIT: f64 = 0.5;
                        let clamp = |new: f64, old: f64| {
                            new.clamp(old - LIMIT, old + LIMIT)
                        };
                        previous.v2 = clamp(vs[1], previous.v2);
                        previous.v3 = clamp(vs[2], previous.v3);
                        previous.v = vs[0];
                    }
                    _ => {}
                }
            }
            iterations += 1;
            self.total_iterations += 1;
            if nonlinear_indices.is_empty() || max_change <= self.settings.convergence_tolerance_v {
                self.last_step_iterations = iterations;
                self.scratch = matrix;
                return self.commit(working, &solution, &voltages, &assembly, dt);
            }
            if iterations >= self.settings.max_iterations {
                self.scratch = matrix;
                return Err(Diagnostic::NonConvergence {
                    iterations,
                    change_v: max_change,
                });
            }
        }
    }

    pub fn step(&mut self, requested_dt: f64, steps: u64) -> StepOutcome {
        let mut outcome = StepOutcome::default();
        self.step_diagnostics.clear();
        if !requested_dt.is_finite() || requested_dt <= 0.0 {
            outcome.failed = true;
            outcome.diagnostics.push(Diagnostic::InvalidValue {
                element_id: "step".to_string(),
                parameter: "dt_s".to_string(),
            });
            return outcome;
        }
        if !self.started {
            self.current_dt = requested_dt
                .min(self.settings.max_dt_s)
                .max(self.settings.min_dt_s);
            self.started = true;
        } else {
            self.current_dt = self
                .current_dt
                .min(self.settings.max_dt_s)
                .max(self.settings.min_dt_s);
        }

        for _ in 0..steps {
            if self.perf.enabled {
                self.perf.steps += 1;
            }
            let saved_dt = self.current_dt;
            let mut dt = self.current_dt;
            let mut accepted = false;
            for _ in 0..=self.settings.max_halvings {
                match self.attempt_step(dt) {
                    Ok(()) => {
                        accepted = true;
                        outcome.steps_done += 1;
                        outcome.advanced_s += dt;
                        self.current_dt = dt;
                        break;
                    }
                    Err(diagnostic) => {
                        self.record_step_diagnostic(diagnostic.clone());
                        outcome.diagnostics.push(diagnostic);
                        dt /= 2.0;
                        if dt < self.settings.min_dt_s {
                            break;
                        }
                    }
                }
            }
            if !accepted {
                self.current_dt = saved_dt;
                outcome.failed = true;
                break;
            }
            self.good_steps += 1;
            if self.good_steps >= self.settings.good_steps_before_growth {
                self.current_dt = (self.current_dt * 2.0).min(self.settings.max_dt_s);
                self.good_steps = 0;
            }
        }
        outcome
    }
}

impl Default for Circuit {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OHM: &str = "resistance_ohm";
    const VOLT: &str = "voltage_v";
    const CAP: &str = "capacitance_f";
    const IND: &str = "inductance_h";
    const IS: &str = "is_a";

    fn op_add(id: &str, kind: &str, nodes: &[&str]) -> Operation {
        Operation::Add {
            id: id.to_string(),
            kind: kind.to_string(),
            nodes: nodes.iter().map(|n| n.to_string()).collect(),
        }
    }

    fn param(circuit: &mut Circuit, id: &str, key: &str, value: f64) {
        circuit.set_parameter(id, key, value).unwrap();
    }

    fn divider() -> Circuit {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("v1", "voltage_source", &["n1", "gnd"]),
                op_add("r1", "resistor", &["n1", "n2"]),
                op_add("r2", "resistor", &["n2", "gnd"]),
            ])
            .unwrap();
        param(&mut circuit, "v1", VOLT, 10.0);
        param(&mut circuit, "r1", OHM, 1000.0);
        param(&mut circuit, "r2", OHM, 1000.0);
        circuit
    }

    fn rc() -> Circuit {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("v1", "voltage_source", &["n1", "gnd"]),
                op_add("r1", "resistor", &["n1", "n2"]),
                op_add("c1", "capacitor", &["n2", "gnd"]),
            ])
            .unwrap();
        param(&mut circuit, "v1", VOLT, 10.0);
        param(&mut circuit, "r1", OHM, 1000.0);
        param(&mut circuit, "c1", CAP, 1e-6);
        circuit
    }

    fn rl() -> Circuit {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("v1", "voltage_source", &["n1", "gnd"]),
                op_add("r1", "resistor", &["n1", "n2"]),
                op_add("l1", "inductor", &["n2", "gnd"]),
            ])
            .unwrap();
        param(&mut circuit, "v1", VOLT, 10.0);
        param(&mut circuit, "r1", OHM, 100.0);
        param(&mut circuit, "l1", IND, 0.1);
        circuit
    }

    fn diode_circuit() -> Circuit {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("v1", "voltage_source", &["n1", "gnd"]),
                op_add("r1", "resistor", &["n1", "n2"]),
                op_add("d1", "diode", &["n2", "gnd"]),
            ])
            .unwrap();
        param(&mut circuit, "v1", VOLT, 5.0);
        param(&mut circuit, "r1", OHM, 1000.0);
        param(&mut circuit, "d1", IS, 1e-12);
        circuit
    }

    fn voltage(circuit: &Circuit, node: &str) -> f64 {
        circuit
            .node_voltages()
            .into_iter()
            .find(|(id, _)| id == node)
            .unwrap()
            .1
    }

    #[test]
    fn stamps_matrix_and_solves_known_system() {
        let mut matrix = Matrix::default();
        matrix.reset(3, 3);
        matrix.stamp_conductance(1, 2, 0.01);
        matrix.stamp_voltage_source(1, 0, 10.0, 0);
        let solution = lu_solve_in_place(&mut matrix.a, &mut matrix.b).unwrap();
        assert!((solution[0] - 10.0).abs() < 1e-9);
        assert!((solution[1] - 10.0).abs() < 1e-9);
        assert!(solution[2].abs() < 1e-9);
    }

    #[test]
    fn singular_matrix_is_diagnosed() {
        let mut matrix = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        let mut rhs = vec![1.0, 2.0];
        assert_eq!(
            lu_solve_in_place(&mut matrix, &mut rhs),
            Err(Diagnostic::SingularMatrix)
        );
    }

    #[test]
    fn resistive_divider_matches_analytic_result() {
        let mut circuit = divider();
        assert!(circuit.diagnostics.is_empty());
        let outcome = circuit.step(5e-6, 1);
        assert!(!outcome.failed);
        assert!((voltage(&circuit, "n2") - 5.0).abs() < 1e-9);
        assert!((circuit.measurement("r1", "current").unwrap() - 0.005).abs() < 1e-9);
    }

    #[test]
    fn high_value_resistor_divider_is_not_singular() {
        let mut circuit = divider();
        param(&mut circuit, "r1", OHM, 1e11);
        param(&mut circuit, "r2", OHM, 1e11);
        let outcome = circuit.step(5e-6, 1);
        assert!(!outcome.failed, "high-impedance divider must solve: {:?}", outcome.diagnostics);
        assert!((voltage(&circuit, "n2") - 5.0).abs() < 1e-9);
    }

    #[test]
    fn rc_transient_matches_analytic_time_constant() {
        let mut circuit = rc();
        let outcome = circuit.step(5e-6, 200);
        assert!(!outcome.failed);
        assert!((circuit.time_s() - 1e-3).abs() < 1e-12);
        let expected = 10.0 * (1.0 - (-1.0_f64).exp());
        assert!((voltage(&circuit, "n2") - expected).abs() < 0.05);
    }

    #[test]
    fn rl_transient_matches_analytic_time_constant() {
        let mut circuit = rl();
        let outcome = circuit.step(5e-6, 200);
        assert!(!outcome.failed);
        let expected = (10.0 / 100.0) * (1.0 - (-1.0_f64).exp());
        let current = circuit.measurement("l1", "current").unwrap();
        assert!((current - expected).abs() < 0.005);
    }

    #[test]
    fn diode_converges_to_shockley_operating_point() {
        let mut circuit = diode_circuit();
        let outcome = circuit.step(5e-6, 200);
        assert!(!outcome.failed, "diode step must converge: {:?}", outcome.diagnostics);
        let diode_voltage = voltage(&circuit, "n2");
        let current = circuit.measurement("d1", "current").unwrap();
        assert!((diode_voltage - 0.576).abs() < 0.02);
        assert!((current - 0.00442).abs() < 0.0002);
    }

    #[test]
    fn floating_nodes_and_singular_matrices_are_reported() {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("r1", "resistor", &["n1", "n2"]),
            ])
            .unwrap();
        assert!(circuit
            .diagnostics
            .iter()
            .any(|d| matches!(d, Diagnostic::FloatingNode { .. })));
        let outcome = circuit.step(5e-6, 1);
        assert!(outcome.failed);
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| matches!(d, Diagnostic::SingularMatrix)));
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        let mut circuit = divider();
        assert!(matches!(
            circuit.set_parameter("r1", OHM, 0.0),
            Err(Diagnostic::InvalidValue { .. })
        ));
        assert!(matches!(
            circuit.set_parameter("nope", OHM, 1.0),
            Err(Diagnostic::InvalidValue { .. })
        ));
    }

    #[test]
    fn parameter_edit_refreshes_diagnostics() {
        let mut circuit = divider();
        circuit
            .elements
            .iter_mut()
            .find(|e| e.id == "r1")
            .unwrap()
            .params
            .insert(OHM.to_string(), 0.0);
        circuit.refresh_diagnostics();
        assert!(circuit.diagnostics.iter().any(
            |d| matches!(d, Diagnostic::InvalidValue { parameter, .. } if parameter == OHM)
        ));
        param(&mut circuit, "r1", OHM, 1000.0);
        assert!(circuit.diagnostics.is_empty());
    }

    #[test]
    fn rejected_step_restores_previous_accepted_state() {
        let mut circuit = diode_circuit();
        circuit.settings.max_iterations = 1;
        let before = circuit.node_voltages();
        let outcome = circuit.step(5e-6, 1);
        assert!(outcome.failed);
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| matches!(d, Diagnostic::NonConvergence { .. })));
        assert_eq!(circuit.node_voltages(), before);
        assert_eq!(circuit.time_s(), 0.0);
    }

    #[test]
    fn adaptive_dt_persists_across_step_calls() {
        let mut batched = divider();
        let mut separate = divider();
        for circuit in [&mut batched, &mut separate] {
            circuit.settings.max_dt_s = 1e-3;
        }
        batched.step(5e-6, 3);
        for _ in 0..3 {
            separate.step(5e-6, 1);
        }
        assert!((separate.current_dt() - batched.current_dt()).abs() < 1e-15);
        assert!((separate.current_dt() - 1e-5).abs() < 1e-15);
        separate.reset();
        assert!(!separate.started);
        assert_eq!(separate.current_dt(), DEFAULT_DT_S);
    }

    #[test]
    fn step_diagnostics_do_not_accumulate_across_calls() {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("r1", "resistor", &["n1", "n2"]),
            ])
            .unwrap();
        for _ in 0..100 {
            circuit.step(5e-6, 1);
        }
        assert!(circuit.step_diagnostics.len() <= MAX_STEP_DIAGNOSTICS);
        assert!(circuit.all_diagnostics().len() <= MAX_STEP_DIAGNOSTICS + circuit.diagnostics.len());
    }

    #[test]
    fn successful_step_clears_prior_step_diagnostics() {
        let mut circuit = diode_circuit();
        circuit.settings.max_iterations = 1;
        circuit.step(5e-6, 1);
        assert!(!circuit.step_diagnostics.is_empty());
        circuit.settings.max_iterations = 100;
        circuit.step(5e-6, 1);
        assert!(circuit.step_diagnostics.is_empty());
    }

    #[test]
    fn repeated_runs_are_deterministic() {
        let mut first = diode_circuit();
        let mut second = diode_circuit();
        first.step(5e-6, 20);
        second.step(5e-6, 20);
        assert_eq!(first.node_voltages(), second.node_voltages());
        assert_eq!(first.time_s(), second.time_s());
    }

    #[test]
    fn wires_merge_nodes() {
        let mut circuit = divider();
        circuit
            .apply_operations(vec![op_add("w1", "wire", &["n2", "n3"])])
            .unwrap();
        assert!(circuit.node_ids.contains(&"n2".to_string()));
        assert!(!circuit.node_ids.contains(&"n3".to_string()));
        let wire = circuit.elements.iter().find(|e| e.id == "w1").unwrap();
        assert_eq!(wire.nodes, vec!["n2", "n2"]);
        circuit.step(5e-6, 1);
        assert!((voltage(&circuit, "n2") - 5.0).abs() < 1e-9);
    }

    #[test]
    fn switch_conducts_only_when_closed() {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("v1", "voltage_source", &["n1", "gnd"]),
                op_add("r1", "resistor", &["n1", "n2"]),
                op_add("sw1", "switch", &["n2", "gnd"]),
            ])
            .unwrap();
        param(&mut circuit, "v1", VOLT, 10.0);
        param(&mut circuit, "r1", OHM, 1000.0);
        param(&mut circuit, "sw1", "resistance_ohm", 1e-3);
        circuit.set_parameter("sw1", "closed", 0.0).unwrap();
        assert!(!circuit.step(5e-6, 5).failed);
        let open_current = circuit.measurement("r1", "current").unwrap();
        assert!(open_current.abs() < 1e-9);
        assert!((voltage(&circuit, "n2") - 10.0).abs() < 1e-9);
        circuit.set_parameter("sw1", "closed", 1.0).unwrap();
        assert!(!circuit.step(5e-6, 5).failed);
        let closed_current = circuit.measurement("r1", "current").unwrap();
        assert!(closed_current > 0.0);
        assert!((voltage(&circuit, "n2")).abs() < 0.01);
    }

    #[test]
    fn switch_toggle_rejects_invalid_binary_value() {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("sw1", "switch", &["n1", "gnd"]),
            ])
            .unwrap();
        assert!(matches!(
            circuit.set_parameter("sw1", "closed", 0.5),
            Err(Diagnostic::InvalidValue { .. })
        ));
    }

    #[test]
    fn bjt_converges_and_scales_with_beta() {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("v1", "voltage_source", &["n1", "gnd"]),
                op_add("r1", "resistor", &["n1", "n2"]),
                op_add("r2", "resistor", &["n1", "n3"]),
                op_add("q1", "bjt", &["n2", "n3", "gnd"]),
            ])
            .unwrap();
        param(&mut circuit, "v1", VOLT, 10.0);
        param(&mut circuit, "r1", OHM, 1000000.0);
        param(&mut circuit, "r2", OHM, 1000.0);
        param(&mut circuit, "q1", "beta", 100.0);
        param(&mut circuit, "q1", "is_a", 1e-12);
        let outcome = circuit.step(5e-6, 100);
        assert!(!outcome.failed, "bjt step must converge: {:?}", outcome.diagnostics);
        let base_voltage = voltage(&circuit, "n2");
        let collector_voltage = voltage(&circuit, "n3");
        assert!(base_voltage > 0.4 && base_voltage < 0.8);
        let ic = circuit.measurement("q1", "current").unwrap();
        assert!(ic > 0.0);
        let ib_recomputed = 1e-12 * ((base_voltage / 0.025865).exp() - 1.0);
        assert!((ic - 100.0 * ib_recomputed).abs() < 1e-4);
        assert!(collector_voltage > 0.0 && collector_voltage < 10.0);
    }

    #[test]
    fn mosfet_linear_region_matches_analytic_ids() {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("vg", "voltage_source", &["ng", "gnd"]),
                op_add("vd", "voltage_source", &["nd", "gnd"]),
                op_add("m1", "mosfet", &["ng", "gnd", "nd"]),
            ])
            .unwrap();
        param(&mut circuit, "vg", VOLT, 5.0);
        param(&mut circuit, "vd", VOLT, 3.0);
        param(&mut circuit, "m1", "vt", 1.5);
        param(&mut circuit, "m1", "beta", 0.02);
        let outcome = circuit.step(5e-6, 50);
        assert!(!outcome.failed, "mosfet step must converge: {:?}", outcome.diagnostics);
        let vds = voltage(&circuit, "nd") - voltage(&circuit, "gnd");
        let vgs = voltage(&circuit, "ng") - voltage(&circuit, "gnd");
        // linear region: ids = beta * ((vgs - vt) * vds - vds^2 / 2)
        let expected = 0.02 * ((5.0 - 1.5) * 3.0 - 3.0 * 3.0 / 2.0);
        assert!((vds - 3.0).abs() < 1e-9);
        assert!((vgs - 5.0).abs() < 1e-9);
        let ids = circuit.measurement("m1", "current").unwrap();
        assert!((ids - expected).abs() < 0.005, "ids={ids}, expected={expected}");
    }

    #[test]
    fn vactrol_transfer_matches_documented_checkpoints() {
        let element = Element::new(
            "v".to_string(),
            "vactrol".to_string(),
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ],
        );
        let resistance = |current| 1.0 / Circuit::vactrol_target_conductance(&element, current);
        assert!((resistance(0.0) - 10e6).abs() < 1.0);
        assert!((resistance(1e-3) - 30e3).abs() < 1.0);
        assert!((resistance(10e-3) - 4.75e3).abs() < 100.0);
        assert!((resistance(40e-3) - 1.57e3).abs() < 30.0);
        assert!(resistance(100.0) >= 1.5e3);
        assert!(resistance(-1.0) >= 10e6 - 1.0);
    }

    #[test]
    fn vactrol_conductance_update_is_exact_and_bounded() {
        let element = Element::new(
            "v".to_string(),
            "vactrol".to_string(),
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ],
        );
        let from = 1e-7;
        let to = 1e-4;
        let on = element.params.get("turn_on_s").copied().unwrap();
        let advanced = Circuit::vactrol_advance_conductance(&element, from, to, on);
        let expected = from + (to - from) * (1.0 - (-1.0_f64).exp());
        assert!((advanced - expected).abs() < 1e-12);
        assert!(Circuit::vactrol_advance_conductance(&element, from, to, 10.0) <= to);
        assert!(Circuit::vactrol_advance_conductance(&element, to, from, 10.0) >= from);
    }

    #[test]
    fn mosfet_state_stores_gate_voltage() {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("vg", "voltage_source", &["ng", "gnd"]),
                op_add("vd", "voltage_source", &["nd", "gnd"]),
                op_add("m1", "mosfet", &["ng", "gnd", "nd"]),
            ])
            .unwrap();
        param(&mut circuit, "vg", VOLT, 5.0);
        param(&mut circuit, "vd", VOLT, 3.0);
        param(&mut circuit, "m1", "vt", 1.5);
        param(&mut circuit, "m1", "beta", 0.02);
        circuit.step(5e-6, 1);
        let stored = circuit.state.elements.get("m1").unwrap().v;
        assert!((stored - 5.0).abs() < 1e-9, "state.v must hold gate voltage, got {stored}");
    }

    #[test]
    fn unsupported_kind_is_rejected_at_add_time() {
        let mut circuit = Circuit::new();
        let outcome = circuit.apply_operations(vec![op_add("bad", "vacuum_tube", &["n1", "n2"])]);
        assert!(matches!(
            outcome,
            Err(Diagnostic::InvalidValue { parameter, .. }) if parameter == "kind"
        ));
    }

    #[test]
    #[ignore = "benchmark harness; run with --ignored --nocapture"]
    fn benchmark_divider_chain() {
        let mut circuit = Circuit::new();
        circuit
            .apply_operations(vec![
                op_add("ground", "ground", &["gnd"]),
                op_add("v1", "voltage_source", &["n0", "gnd"]),
            ])
            .unwrap();
        param(&mut circuit, "v1", VOLT, 10.0);
        for i in 0..40 {
            let id = format!("r{i}");
            circuit
                .apply_operations(vec![op_add(
                    &id,
                    "resistor",
                    &[&format!("n{i}"), &format!("n{}", i + 1)],
                )])
                .unwrap();
            param(&mut circuit, &id, OHM, 1000.0);
        }
        circuit
            .apply_operations(vec![op_add("rlast", "resistor", &["n40", "gnd"])])
            .unwrap();
        param(&mut circuit, "rlast", OHM, 1000.0);

        let started = std::time::Instant::now();
        let outcome = circuit.step(5e-6, 1000);
        println!(
            "40-resistor chain, 1000 steps: {:?} (failed={})",
            started.elapsed(),
            outcome.failed
        );
        assert!(!outcome.failed);
    }
}
