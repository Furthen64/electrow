# Component models (Task 04)

Implemented: wires, ground, resistors, independent voltage/current sources, switches, capacitors, inductors, diodes, simplified BJTs and MOSFETs. The model registry in `engine/src/models.rs` is the single source of truth for supported kinds, terminal mapping, parameter defaults and validation rules; the solver consumes it in `engine/src/solver.rs`.

All models are idealized; they are not SPICE equivalence. Nonlinear models use the same Newton-style iteration as the diode: clamp per-step voltage changes, iterate until `|ΔV| <= convergence_tolerance_v` or `max_iterations` is hit, roll back on failure.

## Terminal mapping and conventions

Conventions: positive current flows from terminal 0 to terminal 1 (or across the junction as documented). Voltages are terminal0 minus terminal1 (where applicable). Ground is node index 0.

| Kind | Terminals (in order) | Description |
|---|---|---|
| `wire` | a, b | Merges a and b into one node |
| `ground` | gnd | Node reference for the whole circuit |
| `resistor` | a, b | `I = (Va - Vb) / R` |
| `voltage_source` | a, b | Enforces `Va - Vb = voltage_v` |
| `current_source` | a, b | Forces `I = current_a` from a to b |
| `capacitor` | a, b | Trapezoidal companion model |
| `inductor` | a, b | Trapezoidal companion model |
| `diode` | anode, cathode | Shockley model (forward-only) |
| `switch` | a, b | Ideal switch: closed = small resistance, open = no connection |
| `bjt` | base, collector, emitter | Simplified BJT, Shockley BE + beta-controlled collector current |
| `mosfet` | gate, source, drain | Simplified square-law MOSFET, no body diodes |

## Parameter defaults and validation

| Kind | Parameter | Default | Validation |
|---|---|---|---|
| `resistor` | `resistance_ohm` | 1000.0 | positive |
| `voltage_source` | `voltage_v` | 5.0 | finite |
| `current_source` | `current_a` | 1e-3 | finite |
| `capacitor` | `capacitance_f` | 1e-6 | positive |
| `inductor` | `inductance_h` | 1e-3 | positive |
| `diode` | `is_a` | 1e-12 | positive |
| `diode` | `emission` | 1.0 | positive |
| `switch` | `closed` | 1.0 | binary (0 or 1) |
| `switch` | `resistance_ohm` | 1e-3 | positive (used when closed) |
| `bjt` | `beta` | 100.0 | positive |
| `bjt` | `is_a` | 1e-12 | positive |
| `bjt` | `emission` | 1.0 | positive |
| `bjt` | `pnp` | 0.0 | binary (0 = NPN, 1 = PNP) |
| `mosfet` | `pnp` | 0.0 | binary (0 = NMOS, 1 = PMOS) |
| `mosfet` | `vt` | 1.5 | finite |
| `mosfet` | `beta` | 0.02 | positive |

Unsupported kinds are rejected at `add` time with `invalid_value` / `kind`. Unsupported parameter keys are accepted by `set_parameter` (existing contract) but have no effect.

## Per-model behavior

### `switch`
Closed: stamps `G = 1 / resistance_ohm` (default 1e-3 ohm). Open: no stamp (no connection). Current measurement: `(Va - Vb) / R` when closed, else 0. Toggling via `set_parameter("closed", 0.0 | 1.0)`; only binary values are accepted.

### `bjt` (simplified)
NPN/PNP via `pnp`. Physical forward-bias junction voltage: `vj = pn * (Vb - Ve)` with `pn = +1` (NPN) or `-1` (PNP). Shockley base-emitter: `Ib = is_a * (exp(vj / Vt) - 1)`, `Vt = emission * 0.025865`, exponent clamped to [-100, 50]. Collector current: `Ic = beta * Ib` (active region only, no saturation or Early effect).

Stamping: BE companion conductance `g = is * exp(vj/Vt) / Vt` between base and emitter; a current source for `Ib` (base→emitter for NPN, emitter→base for PNP) and a current source for `Ic` (collector→emitter for NPN, emitter→collector for PNP). `current` measurement: `Ic`. `voltage` measurement: `Vc - Ve`.

Limits: no saturation model (Ic grows with Ib even when Vc < Ve), no collector resistance, beta is DC only. Not PSpice-equivalent.

### `mosfet` (simplified)
Falstad-style square-law model. Terminal order: gate, source, drain. Nodes may be swapped automatically (Falstad convention): when the algebraically closer terminal would swap, source and drain are exchanged before computing `vgs = Vg - Vs` and `vds = Vd - Vs`. `pn = +1` for NMOS, `-1` for PMOS; source/drain are selected by comparing `pn * vs[source_initial]` vs `pn * vs[drain_initial]`.

- Cutoff (`vgs < vt`): `Gds = 1e-8`, `ids = vds * Gds`
- Linear (`vds < vgs - vt`): `ids = beta * ((vgs - vt) * vds - vds^2 / 2)`, `gm = beta * vds`, `Gds = beta * (vgs - vds - vt)`
- Saturation (`vds >= vgs - vt`): `gm = beta * (vgs - vt)`, `ids = 0.5 * beta * (vgs - vt)^2 + (vds - (vgs - vt)) * 1e-8`

Nonlinear iteration clamps source/drain voltage changes to ±0.5 V per step; convergence is checked against 10 mV (scaled by 100 when `beta > 1`, matching Falstad's stricter threshold). Gate voltage is not clamped. `current` measurement: `ids` (after source/drain swap). `voltage` measurement: `Vd - Vs` (physical drain-source).

Limits: no body diodes, no bulk terminal, no `rds` model, no avalanche/gilbert/trap effects.

## Tests

Regression tests live in `engine/src/solver.rs` and `engine/src/models.rs`.

- `switch_conducts_only_when_closed`: open switch blocks current (0 V across resistor); closed switch conducts with `R = 1e-3`.
- `switch_toggle_rejects_invalid_binary_value`: `closed = 0.5` rejected.
- `bjt_converges_and_scales_with_beta`: NPN driven from 1 MΩ base resistor; `Vb` in [0.4, 0.8] V and `Ic ≈ beta * Ib` consistent with the Shockley equation.
- `mosfet_linear_region_matches_analytic_ids`: NMOS in linear region (VGS = 5 V, VDS = 3 V, vt = 1.5, beta = 0.02); `ids` matches the analytic `beta * ((vgs - vt) * vds - vds^2 / 2)` within 5 mA.
- `unsupported_kind_is_rejected_at_add_time`: unknown kinds are rejected by `apply_operations`.
- `models::tests::registry_covers_supported_kinds`: the registry lists every supported kind.

## Open questions / deferred

- Controlled sources (VCCS/CCCS), op-amps, digital gates: not in this slice; deferred pending demand.
- Body diodes and source/drain swaps for MOSFET: the swap heuristic is implemented, but bulk depletion and reverse-bias body diodes are not.
- BJT saturation and early effects: not implemented.
- Saturation-limited transconductance / gate capacitances: not implemented.
