//! Model registry: single source of truth for supported element kinds.
//!
//! The registry feeds validation, parameter defaults and the metadata the UI
//! editor uses. Solver behavior lives in `solver.rs`; keep this module free of
//! numeric logic so the same metadata can be reused by docs/tooling.

pub struct ParamSpec {
    pub key: &'static str,
    pub default: f64,
    pub positive: bool,
    pub binary: bool,
    pub description: &'static str,
}

pub struct ModelSpec {
    pub kind: &'static str,
    pub terminals: &'static [&'static str],
    pub nonlinear: bool,
    pub description: &'static str,
    pub params: &'static [ParamSpec],
}

static SWITCH_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        key: "closed",
        default: 1.0,
        positive: false,
        binary: true,
        description: "1.0 = closed, 0.0 = open",
    },
    ParamSpec {
        key: "resistance_ohm",
        default: 1e-3,
        positive: true,
        binary: false,
        description: "Resistance when closed (ohms)",
    },
];

static BJT_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        key: "beta",
        default: 100.0,
        positive: true,
        binary: false,
        description: "DC current gain",
    },
    ParamSpec {
        key: "is_a",
        default: 1e-12,
        positive: true,
        binary: false,
        description: "Base-emitter saturation current (A)",
    },
    ParamSpec {
        key: "emission",
        default: 1.0,
        positive: true,
        binary: false,
        description: "Emission coefficient (dimensionless)",
    },
    ParamSpec {
        key: "pnp",
        default: 0.0,
        positive: false,
        binary: true,
        description: "0.0 = NPN, 1.0 = PNP",
    },
];

static MOSFET_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        key: "pnp",
        default: 0.0,
        positive: false,
        binary: true,
        description: "0.0 = NMOS, 1.0 = PMOS",
    },
    ParamSpec {
        key: "vt",
        default: 1.5,
        positive: false,
        binary: false,
        description: "Threshold voltage (V)",
    },
    ParamSpec {
        key: "beta",
        default: 0.02,
        positive: true,
        binary: false,
        description: "Transconductance scale (A/V²)",
    },
];

static RESISTOR_PARAMS: &[ParamSpec] = &[ParamSpec {
    key: "resistance_ohm",
    default: 1000.0,
    positive: true,
    binary: false,
    description: "Resistance (ohms)",
}];

static VOLTAGE_SOURCE_PARAMS: &[ParamSpec] = &[ParamSpec {
    key: "voltage_v",
    default: 5.0,
    positive: false,
    binary: false,
    description: "Voltage (V)",
}];

static CURRENT_SOURCE_PARAMS: &[ParamSpec] = &[ParamSpec {
    key: "current_a",
    default: 1e-3,
    positive: false,
    binary: false,
    description: "Current (A)",
}];

static CAPACITOR_PARAMS: &[ParamSpec] = &[ParamSpec {
    key: "capacitance_f",
    default: 1e-6,
    positive: true,
    binary: false,
    description: "Capacitance (F)",
}];

static INDUCTOR_PARAMS: &[ParamSpec] = &[ParamSpec {
    key: "inductance_h",
    default: 1e-3,
    positive: true,
    binary: false,
    description: "Inductance (H)",
}];

static VACTROL_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        key: "dark_resistance_ohm",
        default: 10e6,
        positive: true,
        binary: false,
        description: "LED-off cell resistance (ohms)",
    },
    ParamSpec {
        key: "resistance_at_1ma_ohm",
        default: 30e3,
        positive: true,
        binary: false,
        description: "Cell resistance at 1 mA LED current (ohms)",
    },
    ParamSpec {
        key: "response_exponent",
        default: 0.8,
        positive: true,
        binary: false,
        description: "Power-law exponent of the LED/cell transfer curve",
    },
    ParamSpec {
        key: "minimum_resistance_ohm",
        default: 1.5e3,
        positive: true,
        binary: false,
        description: "Clamped minimum cell resistance (ohms)",
    },
    ParamSpec {
        key: "turn_on_s",
        default: 2.5e-3,
        positive: true,
        binary: false,
        description: "Conductance rise time constant (s)",
    },
    ParamSpec {
        key: "turn_off_s",
        default: 8.403422185645153e-3,
        positive: true,
        binary: false,
        description: "Conductance decay time constant (s)",
    },
];

static DIODE_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        key: "is_a",
        default: 1e-12,
        positive: true,
        binary: false,
        description: "Saturation current (A)",
    },
    ParamSpec {
        key: "emission",
        default: 1.0,
        positive: true,
        binary: false,
        description: "Emission coefficient (dimensionless)",
    },
];

pub static MODELS: &[ModelSpec] = &[
    ModelSpec {
        kind: "wire",
        terminals: &["a", "b"],
        nonlinear: false,
        description: "Ideal wire",
        params: &[],
    },
    ModelSpec {
        kind: "ground",
        terminals: &["gnd"],
        nonlinear: false,
        description: "Ground reference",
        params: &[],
    },
    ModelSpec {
        kind: "resistor",
        terminals: &["a", "b"],
        nonlinear: false,
        description: "Ideal resistor",
        params: RESISTOR_PARAMS,
    },
    ModelSpec {
        kind: "voltage_source",
        terminals: &["a", "b"],
        nonlinear: false,
        description: "Ideal independent voltage source",
        params: VOLTAGE_SOURCE_PARAMS,
    },
    ModelSpec {
        kind: "current_source",
        terminals: &["a", "b"],
        nonlinear: false,
        description: "Ideal independent current source",
        params: CURRENT_SOURCE_PARAMS,
    },
    ModelSpec {
        kind: "capacitor",
        terminals: &["a", "b"],
        nonlinear: false,
        description: "Ideal capacitor (trapezoidal)",
        params: CAPACITOR_PARAMS,
    },
    ModelSpec {
        kind: "inductor",
        terminals: &["a", "b"],
        nonlinear: false,
        description: "Ideal inductor (trapezoidal)",
        params: INDUCTOR_PARAMS,
    },
    ModelSpec {
        kind: "diode",
        terminals: &["anode", "cathode"],
        nonlinear: true,
        description: "Ideal diode (Shockley, forward-only)",
        params: DIODE_PARAMS,
    },
    ModelSpec {
        kind: "vactrol",
        terminals: &["led_anode", "led_cathode", "cell_a", "cell_b"],
        nonlinear: true,
        description: "Four-terminal resistive optocoupler with first-order cell conductance",
        params: VACTROL_PARAMS,
    },
    ModelSpec {
        kind: "switch",
        terminals: &["a", "b"],
        nonlinear: false,
        description: "Ideal switch (closed = small resistance, open = no connection)",
        params: SWITCH_PARAMS,
    },
    ModelSpec {
        kind: "bjt",
        terminals: &["base", "collector", "emitter"],
        nonlinear: true,
        description: "Simplified BJT: Shockley BE + beta-controlled collector current",
        params: BJT_PARAMS,
    },
    ModelSpec {
        kind: "mosfet",
        terminals: &["gate", "source", "drain"],
        nonlinear: true,
        description: "Simplified MOSFET: square-law (off/linear/saturation), no body diodes",
        params: MOSFET_PARAMS,
    },
];

pub fn model_spec(kind: &str) -> Option<&'static ModelSpec> {
    MODELS.iter().find(|spec| spec.kind == kind)
}

pub fn supported_kinds() -> Vec<&'static str> {
    MODELS.iter().map(|spec| spec.kind).collect()
}

impl ModelSpec {
    pub fn param(&self, key: &str) -> Option<&'static ParamSpec> {
        self.params.iter().find(|param| param.key == key)
    }

    pub fn default_params(&self) -> std::collections::HashMap<String, f64> {
        self.params
            .iter()
            .map(|param| (param.key.to_string(), param.default))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_covers_supported_kinds() {
        for kind in [
            "wire", "ground", "resistor", "voltage_source", "current_source", "capacitor",
            "inductor", "diode", "switch", "bjt", "mosfet", "vactrol",
        ] {
            assert!(model_spec(kind).is_some(), "missing {kind}");
        }
    }

    #[test]
    fn unsupported_kind_returns_none() {
        assert!(model_spec("vacuum_tube").is_none());
    }

    #[test]
    fn every_param_has_a_default() {
        for spec in MODELS {
            for param in spec.params {
                assert!(param.default.is_finite());
            }
        }
    }
}

pub fn check_param_value(spec: &ParamSpec, value: f64) -> Option<&'static str> {
    if !value.is_finite() {
        return Some("must be finite");
    }
    if spec.binary && value != 0.0 && value != 1.0 {
        return Some("must be 0.0 or 1.0");
    }
    if spec.positive && value <= 0.0 {
        return Some("must be positive");
    }
    None
}

pub fn param_spec(kind: &str, key: &str) -> Option<&'static ParamSpec> {
    model_spec(kind)?.param(key)
}
