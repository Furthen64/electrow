# Solver algorithm and source map (Task 03)

Status: baseline for the Rust port. Written before translation, per Task 03. Reference evidence comes from [reference-analysis.md](reference-analysis.md); no reference source was copied.

## 1. Selected baseline

| Concern | Baseline snapshot | Evidence |
|---|---|---|
| Topology/node objects | Vactrol / LVLabs | `SimulationManager.makeNodeList()` V:`:533-600`, L:`:529-596`; `CircuitNode.java` |
| Stamping API | Vactrol / LVLabs | `SimulationManager.stampResistor` V:`:1183-1194`, `stampVoltageSource` V:`:1157-1163`, `stampConductance` V:`:1196-1201`, `stampMatrix` V:`:1222-1259`, `stampRightSide` V:`:1261-1269` |
| Linear solve | Vactrol / LVLabs dense path | `lu_factor_dense` V:`:1716-1794`, `lu_solve_dense` V:`:1809-1844`; Crout with partial pivoting |
| Nonlinear convergence | Falstad diode model (shared across all three) | `Diode.java:88-181`; `|ΔV| > 0.01` at `:142-143`; `gmin` ramp at `:149-155` |
| Time integration | Vactrol / LVLabs | `CapacitorElm.java:167-169` (`timeStep/(2*C)` trapezoidal vs `timeStep/C` backward Euler); `INTERNALS.md:66-76` |
| Timestep control | Vactrol / LVLabs | `SimulationManager.runCircuit` V:`:1314-1320` doubling after 3 good iterations, `:1390-1407` halving on failure |

SEVA77 is monolithic (`CirSim.java` 7,124 lines) but algorithmically equivalent for these stages: `makeNodeList` `:2465-2524`, `stampCircuit` `:2771-2831`, `runCircuit` `:3262-3420`, `lu_factor` `:6078-6156`. It has dense LU only and no `CircuitNode` objects. The refactored Vactrol/LVLabs form is easier to port without UI coupling, so it is the structural baseline; numerical constants are taken from the shared Falstad code.

## 2. Node numbering and ground convention

- Falstad assigns integer node indices in `makeNodeList`; node **0 is the ground/reference node** (`setGroundNode`, SEVA77 `CirSim.java:2696`). Voltage-source unknowns are appended after node unknowns: matrix size = `nodeCount - 1 + voltageSourceCount` (SEVA77 `:2773`; Vactrol `SimulationManager.java:925`).
- Rust port: after wire merging (union-find), canonical node IDs are collected in first-seen order; the node named `gnd` (or a node owned by a `ground` element) is moved to index 0. If neither exists, the first node is used as reference and an explicit `NoGround` diagnostic is emitted. Nodes not connected to the reference through any element are reported as `FloatingNode`.
- **Deviation:** Falstad ties floating nodes to ground through a `1e8` Ω resistor (SEVA77 `:2592-2598`). The Rust port does not add hidden conductance; it reports the condition instead (Task 03 requirement).

## 3. Stamping conventions

Let `a`, `b` be node indices, with index 0 omitted from the matrix (ground fixed at 0 V).

| Element | Stamp |
|---|---|
| Resistor `R` | `g = 1/R`; `A[a][a] += g`, `A[b][b] += g`, `A[a][b] -= g`, `A[b][a] -= g` |
| Current source `I` (from `a` to `b`) | `rhs[a] -= I`, `rhs[b] += I` |
| Independent voltage source `V` (positive at `a`) | MNA row/column `k`: `A[a][k] += 1`, `A[b][k] -= 1`, `A[k][a] += 1`, `A[k][b] -= 1`, `rhs[k] = V`. Unknown `x[k]` is the current flowing from `a` to `b`. |
| Capacitor (trapezoidal) | `g = 2C/dt`, `Ieq = -g*v_prev - i_prev`; conductance stamp + current source `Ieq` from `a` to `b` |
| Inductor (trapezoidal) | `g = dt/(2L)`, `Ieq = i_prev + g*v_prev`; conductance stamp + current source `Ieq` from `a` to `b` |
| Diode (Newton linearization) | `i = Is*(exp(v/(n*Vt))-1)`, `gd = Is/(n*Vt)*exp(v/(n*Vt))`; `Ieq = i - gd*v_op`; conductance `gd` + current source `Ieq` |

`Vt = 0.025865` V at 27 °C, matching `Diode.java:63`. The exponential argument is clamped to 50 to avoid overflow; this is a numerical guard, not a model change. Diode breakdown is not implemented.

## 4. Linear solve

Dense LU decomposition with partial pivoting, row swaps applied to the RHS, then forward/back substitution. Matches the dense path in `SimulationManager.lu_factor_dense`/`lu_solve_dense`; no iterative solver and no sparse path yet. A pivot magnitude below `1e-12` is treated as singular (reference also rejects zero pivots, `SimulationManager.java:1781-1784`). Matrix and RHS are copied before factorization so the stamped structure remains inspectable and reusable across Newton iterations.

Sparse strategy is explicitly deferred: Vactrol/LVLabs select sparse LU when the largest matrix reaches `SPARSE_THRESHOLD = 150` (`SimulationManager.java:46,1011-1018`). The Rust port starts dense-only; Task 09 will choose from measured circuit sizes.

## 5. Nonlinear iteration

- Newton/Raphson linearization of each diode at the current operating voltage; stop when the largest change in diode voltage is `<= 0.01 V`, matching `Diode.java:142`.
- Explicit step limit of `0.1 V` per iteration on diode voltage updates. Falstad has `limitStep()` (`Diode.java:83-131`); the Rust value is a documented stabilization, not copied.
- Maximum 100 iterations. On exhaustion, the step is rejected, state rolls back, and a `NonConvergence` diagnostic is emitted.
- **Deviation:** no `gmin` conductance ramp (`Diode.java:149-156`) and no BJT gmin logic; those models arrive in Task 04 with their own evidence.

## 6. Time integration and step control

- Trapezoidal companion models only in this slice. Backward Euler is available in the references via `compResistance = timeStep/C`; it is not implemented yet.
- Initial `dt = 5e-6 s`, minimum `5e-11 s`, maximum `5e-6 s`, matching Falstad's `maxTimeStep/minTimeStep` defaults (`CircuitLoader.java:49-50`; SEVA77 `CirSim.java:4384-4385`).
- On a successful step, simulated time increases by the accepted dt. After 3 consecutive accepted steps, dt doubles up to the maximum. On rejection, dt halves and the previous accepted state and time are restored before retrying, up to 8 halvings; the same limit as the reference's timestep-halving loop (`SimulationManager.java:1390-1407`).
- Simulated time is independent of wall-clock cadence and of the number of Newton iterations, matching the Task 02 contract.

## 7. State ownership and rollback

Accepted state (capacitor voltage/current, inductor current/voltage, diode operating voltage, voltage-source branch current) is stored in one `State` map in the Rust `Circuit`. Every attempted step clones this state; a rejected solve restores the clone before retrying. Python and the browser receive only snapshots and never own solver state.

Separate concerns:
- `topology_edit` rebuilds node IDs, ground selection, voltage-source ordering and diagnostics.
- `parameter_edit` changes values only and does not rebuild topology.
- `step` builds a fresh stamped matrix from current state for each attempt.

## 8. Diagnostics

Structured diagnostics emitted by the solver and carried in snapshots:

| Kind | Trigger | Fields |
|---|---|---|
| `NoGround` | No explicit ground element or `gnd` node | `node_id` used as reference |
| `FloatingNode` | Node unreachable from reference through any element | `node_id` |
| `SingularMatrix` | Pivot below threshold or empty rank | — |
| `InvalidValue` | Non-finite matrix entry or invalid parameter | `element_id`, `parameter` |
| `NonConvergence` | Newton iteration limit reached | `iterations`, `change_v` |

Parameter validation rejects non-positive resistance/capacitance/inductance and non-finite source values at edit time, so invalid values cannot silently enter the matrix.

## 9. Scope of this slice

Implemented: wires (node merging), ground, resistors, independent voltage/current sources, capacitors, inductors, diodes; linear solve; trapezoidal transient; Newton iteration; rollback and diagnostics.

Deferred: BJT/MOSFET, controlled sources, op-amps, digital models (Task 04); circuit dump/XML parsing and dumps (Task 05); browser rendering and scopes (Task 07); sparse solver and profiling (Task 09); Vactrol/avalanche/audio (Task 10).

## 10. Default parameters

`topology_edit` adds elements without parameter values, so `Element::new` applies explicit defaults; `parameter_edit` overrides them. Defaults are part of the contract, not hidden stabilization:

| Kind | Defaults |
|---|---|
| `resistor` | `resistance_ohm = 1000` |
| `voltage_source` | `voltage_v = 5` |
| `current_source` | `current_a = 0.001` |
| `capacitor` | `capacitance_f = 1e-6` |
| `inductor` | `inductance_h = 1e-3` |
| `diode` | `is_a = 1e-12`, `emission = 1` |
| `wire`, `ground` | none |

Parameter names are the SI-suffixed keys from `Docs/interface-schema.json`. Unknown keys are accepted by `set_parameter` but do not affect the model; unsupported kinds are rejected at add time.
