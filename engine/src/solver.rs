use std::collections::{HashMap, HashSet};

use crate::json::Value;

pub const DEFAULT_DT_S: f64 = 5e-6;

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

fn default_params(kind: &str) -> HashMap<String, f64> {
    let mut params = HashMap::new();
    match kind {
        "resistor" => {
            params.insert("resistance_ohm".to_string(), 1000.0);
        }
        "voltage_source" => {
            params.insert("voltage_v".to_string(), 5.0);
        }
        "current_source" => {
            params.insert("current_a".to_string(), 1e-3);
        }
        "capacitor" => {
            params.insert("capacitance_f".to_string(), 1e-6);
        }
        "inductor" => {
            params.insert("inductance_h".to_string(), 1e-3);
        }
        "diode" => {
            params.insert("is_a".to_string(), 1e-12);
            params.insert("emission".to_string(), 1.0);
        }
        _ => {}
    }
    params
}

impl Element {
    pub fn new(id: String, kind: String, nodes: Vec<String>) -> Self {
        let params = default_params(&kind);
        Self {
            id,
            kind,
            nodes,
            params,
        }
    }

    pub fn validate(&self) -> Result<(), Diagnostic> {
        let check = |key: &str, positive: bool| -> Result<(), Diagnostic> {
            match self.params.get(key) {
                Some(value) if value.is_finite() && (!positive || *value > 0.0) => Ok(()),
                _ => Err(Diagnostic::InvalidValue {
                    element_id: self.id.clone(),
                    parameter: key.to_string(),
                }),
            }
        };
        match self.kind.as_str() {
            "resistor" => check("resistance_ohm", true),
            "voltage_source" => check("voltage_v", false),
            "current_source" => check("current_a", false),
            "capacitor" => check("capacitance_f", true),
            "inductor" => check("inductance_h", true),
            "diode" => {
                check("is_a", true)?;
                check("emission", true)
            }
            "wire" | "ground" => Ok(()),
            _ => Err(Diagnostic::InvalidValue {
                element_id: self.id.clone(),
                parameter: "kind".to_string(),
            }),
        }
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

struct Matrix {
    a: Vec<Vec<f64>>,
    b: Vec<f64>,
    node_count: usize,
}

impl Matrix {
    fn new(size: usize, node_count: usize) -> Self {
        Self {
            a: vec![vec![0.0; size]; size],
            b: vec![0.0; size],
            node_count,
        }
    }

    fn node_row(&self, node: usize) -> Option<usize> {
        if node == 0 {
            None
        } else {
            Some(node - 1)
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
    None,
}

struct Assembly {
    matrix: Matrix,
    meta: HashMap<String, ElementMeta>,
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
    good_steps: u32,
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

fn lu_solve(matrix: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>, Diagnostic> {
    let size = matrix.len();
    if size == 0 {
        return Ok(Vec::new());
    }
    if matrix.iter().flatten().any(|v| !v.is_finite())
        || rhs.iter().any(|v| !v.is_finite())
    {
        return Err(Diagnostic::InvalidValue {
            element_id: "matrix".to_string(),
            parameter: "value".to_string(),
        });
    }
    let mut a = matrix.to_vec();
    let mut b = rhs.to_vec();
    for k in 0..size {
        let mut pivot = k;
        for row in k + 1..size {
            if a[row][k].abs() > a[pivot][k].abs() {
                pivot = row;
            }
        }
        if a[pivot][k].abs() < 1e-12 {
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
            good_steps: 0,
        }
    }

    pub fn time_s(&self) -> f64 {
        self.time_s
    }

    pub fn current_dt(&self) -> f64 {
        self.current_dt
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
            return Some(voltage);
        }
        if quantity != "current" {
            return None;
        }
        match element.kind.as_str() {
            "resistor" => Some(voltage / element.params.get("resistance_ohm").copied()?),
            "current_source" => Some(element.params.get("current_a").copied()?),
            "wire" => Some(0.0),
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
        Ok(())
    }

    pub fn analyze(&mut self) {
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

        self.diagnostics.clear();
        for element in &self.elements {
            if let Err(diagnostic) = element.validate() {
                self.diagnostics.push(diagnostic);
            }
        }
        if ground.is_some()
            && !self
                .elements
                .iter()
                .any(|e| e.kind == "ground")
                && !self.node_ids.iter().any(|node| node == "gnd")
        {
            self.diagnostics.push(Diagnostic::NoGround {
                node_id: ground.unwrap(),
            });
        }

        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for element in &self.elements {
            if element.nodes.len() >= 2 {
                let a = element.nodes[0].clone();
                let b = element.nodes[1].clone();
                adjacency.entry(a.clone()).or_default().push(b.clone());
                adjacency.entry(b).or_default().push(a);
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
        self.analyze();
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

    fn assemble(&self, state: &State, dt: f64) -> Result<Assembly, Diagnostic> {
        let node_count = self.node_ids.len();
        let size = node_count.saturating_sub(1) + self.voltage_sources.len();
        let mut matrix = Matrix::new(size, node_count);
        let mut meta = HashMap::new();

        for element in &self.elements {
            if element.nodes.len() < 2 {
                meta.insert(element.id.clone(), ElementMeta::None);
                continue;
            }
            let a = *self.node_index.get(&element.nodes[0]).ok_or_else(|| {
                Diagnostic::InvalidValue {
                    element_id: element.id.clone(),
                    parameter: "nodes".to_string(),
                }
            })?;
            let b = *self.node_index.get(&element.nodes[1]).ok_or_else(|| {
                Diagnostic::InvalidValue {
                    element_id: element.id.clone(),
                    parameter: "nodes".to_string(),
                }
            })?;
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
        Ok(Assembly { matrix, meta })
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
        state: &mut State,
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
                _ => {
                    entry.v = voltage;
                }
            }
        }
        self.state = state.clone();
        self.time_s += dt;
        Ok(())
    }

    fn attempt_step(&mut self, dt: f64) -> Result<(), Diagnostic> {
        let mut working = self.state.clone();
        let diode_ids: Vec<String> = self
            .elements
            .iter()
            .filter(|e| e.kind == "diode")
            .map(|e| e.id.clone())
            .collect();
        let mut iterations = 0;
        loop {
            let assembly = self.assemble(&working, dt)?;
            let solution = lu_solve(&assembly.matrix.a, &assembly.matrix.b)?;
            let voltages = self.extract_voltages(&solution);
            let mut max_change = 0.0;
            for id in &diode_ids {
                let element = match self.elements.iter().find(|e| &e.id == id) {
                    Some(element) => element,
                    None => continue,
                };
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
            iterations += 1;
            if diode_ids.is_empty() || max_change <= self.settings.convergence_tolerance_v {
                return self.commit(&mut working, &solution, &voltages, &assembly, dt);
            }
            if iterations >= self.settings.max_iterations {
                return Err(Diagnostic::NonConvergence {
                    iterations,
                    change_v: max_change,
                });
            }
        }
    }

    pub fn step(&mut self, requested_dt: f64, steps: u64) -> StepOutcome {
        let mut outcome = StepOutcome::default();
        if !requested_dt.is_finite() || requested_dt <= 0.0 {
            outcome.failed = true;
            outcome.diagnostics.push(Diagnostic::InvalidValue {
                element_id: "step".to_string(),
                parameter: "dt_s".to_string(),
            });
            return outcome;
        }
        self.current_dt = requested_dt
            .min(self.settings.max_dt_s)
            .max(self.settings.min_dt_s);

        for _ in 0..steps {
            let saved_state = self.state.clone();
            let saved_time = self.time_s;
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
                        self.state = saved_state.clone();
                        self.time_s = saved_time;
                        self.diagnostics.push(diagnostic.clone());
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
        let mut matrix = Matrix::new(3, 3);
        matrix.stamp_conductance(1, 2, 0.01);
        matrix.stamp_voltage_source(1, 0, 10.0, 0);
        let solution = lu_solve(&matrix.a, &matrix.b).unwrap();
        assert!((solution[0] - 10.0).abs() < 1e-9);
        assert!((solution[1] - 10.0).abs() < 1e-9);
        assert!(solution[2].abs() < 1e-9);
    }

    #[test]
    fn singular_matrix_is_diagnosed() {
        let matrix = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        assert_eq!(lu_solve(&matrix, &[1.0, 2.0]), Err(Diagnostic::SingularMatrix));
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
}
