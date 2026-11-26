# Task 01 — Reference source analysis

Status: analysis complete (2026-11-26). Companion machine-readable inventory: [source-inventory.json](source-inventory.json).

This report is read-only. No reference snapshot was modified and no implementation code was copied. Paths below are relative to the repository root; `S:`/`V:`/`L:` prefixes mean `ref_SEVA77/`, `ref_Vactrol/`, `ref_LVLabs/`. Java paths are relative to each snapshot's source root (`S:src/main/java/`, `V:src/`, `L:src/`), which all normalize to `com/lushprojects/circuitjs1/...`.

## 1. Scope and method

- Inspected all three checked-out snapshots listed in [REFERENCES.md](../REFERENCES.md), including their build files, entry points, source trees, docs, tests and packaging.
- Normalized Java paths by stripping the snapshot root and `src/main/java` vs `src`; compared raw-byte SHA-256 hashes and newline counts. Full per-file data for all 256 normalized Java paths is in the JSON inventory.
- Traced call paths in source, not from README claims. Line numbers refer to the current snapshots and will drift if snapshots are refreshed.
- Claim strength: "observed" means direct file evidence; "inferred" is explicitly labelled.

## 2. Snapshot accounting

| Snapshot | Files total | Java under `src/` | Java lines | Version markers | License | Build / runtime |
|---|---:|---:|---:|---|---|---|
| SEVA77 | 680 | 202 | 43,689 | `1.3.2` (`S:pom.xml:4-7`, `S:Inno Setup/setup_x64.iss:5`), `3.1.3js` (`S:.../client/circuitjs1.java:39`) | GPL-2.0-or-later (`S:LICENSE`, `S:README.md:88-96`) | Maven + GWT 2.8.2 + NW.js 0.64.1 + Inno Setup |
| Vactrol | 815 | 251 | 59,481 | upstream `4.1.4js` (`V:.../client/circuitjs1.java:39`), fork `2.2.3` (`V:build.gradle:87-89`), app `0.1.0-beta.1` (`V:app/package.json:4`) | GPL-3.0-or-later combined work (`V:LICENSING.md:5-17`), upstream GPL-2.0-or-later text retained (`V:COPYING.txt`) | Gradle 8.7 + GWT 2.8.2 + Electron 43.4.0 |
| LVLabs | 732 | 243 | 55,824 | `4.1.3js` (`L:.../client/circuitjs1.java:39`), `2.2.3` (`L:build.gradle:86`) | GPL-2.0-or-later (`L:COPYING.txt`, `L:README.md:70-77`); stale GPLv3 header at `L:build.gradle:4` | Gradle 8.7 + GWT 2.8.2, browser-only |

Vactrol has 253 `.java` files in the tree: 251 under `src/` plus two standalone `model-tests/` files (`VactrolModelTest.java`, `BjtAvalancheModelTest.java`).

Source comparison (256 normalized paths in the union):

| Classification | Count | Meaning |
|---|---:|---|
| Common to all three | 198 | 40 byte-identical, 158 differ |
| Shared by Vactrol + LVLabs only | 44 | 29 identical, 15 differ; absent from SEVA77 |
| Unique to SEVA77 | 4 | `HelpDialog`, `LicenseDialog`, `ModDialog`, `RowInfo` |
| Unique to Vactrol | 9 | `VactrolElm`, `VactrolModel`, `BjtAvalancheModel`, `BatteryElm`, `MosfetModel`, `EditMosfetModelDialog`, `KeyNames`, `TestCreator`, `TestManager` |
| Unique to LVLabs | 1 | `Theme.java` |

Representative key files (SHA-256 prefix : newline count; `—` = absent):

| Normalized path | SEVA77 | Vactrol | LVLabs |
|---|---|---|---|
| `client/circuitjs1.java` | `6a6f92d35a26`:194 | `7d13f42206d2`:194 | `6837a1f48fc5`:189 |
| `client/CirSim.java` | `5fd85ae4b3f3`:7124 | `f1d77ee6f16c`:687 | `16f021c42536`:649 |
| `client/CircuitElm.java` | `0fc8fd22b70e`:1220 | `fa3207ee90ea`:1458 | `b6700f368dee`:1413 |
| `client/CircuitMatrix.java` | — | `cd1e508d9a39`:20 | `cd1e508d9a39`:20 |
| `client/CircuitLoader.java` | — | `79a1e4960de5`:310 | `7ef15627c213`:309 |
| `client/SimulationManager.java` | — | `c1d628c4ffd1`:1867 | `3101fbef0eec`:1858 |
| `client/ResistorElm.java` | `45856739ab2d`:121 | `4d47b52111fc`:136 | `4d47b52111fc`:136 |
| `client/VoltageElm.java` | `17e84168ff4b`:374 | `1ca0b28cb637`:688 | `5e2a5af8e0db`:682 |
| `client/DiodeElm.java` | `f5fb5bedec2c`:262 | `6ae4ad3562ca`:282 | `6ae4ad3562ca`:282 |
| `client/Scope.java` | `43f44527a864`:2210 | `5acc1b1179d8`:1474 | `8573e7e9e5a4`:1436 |
| `client/UIManager.java` | — | `ff4dd5832c3d`:1560 | `428e4b2fe166`:1554 |
| `client/matrix/SparseLU.java` | — | `00d71628bd21`:351 | `00d71628bd21`:351 |

`DCVoltageElm.java` (27 lines, `14db2862c147`), `ElementFactory.java` (`0e7226553366`) and `CircuitMatrix.java` are byte-identical where present. `V:`/`L:` `CircuitLoader.java` differ by one line only: `V:.../client/CircuitLoader.java:210` sets `app.consoleExceptionOccurred = true`.

## 3. What Falstad/CircuitJS actually is

Observed architecture:

- A GWT Java-to-JavaScript application. `circuitjs1.java:49` `onModuleLoad()` loads locale data, creates `CirSim`, calls `init()`, then `updateCircuit()` (S:`:179-192`, V:`:179-192`, L:`:174-187`).
- Modified nodal analysis (MNA): each element stamps conductance/current contributions into a matrix; voltage sources add extra rows/columns; the resulting system `A x = B` is solved by LU with partial pivoting. `L:INTERNALS.md:15-24,89-115` documents this explicitly.
- Everything (solver, models, drawing, editing, scopes) runs client-side in the browser. The forks add wrappers: NW.js desktop (S), Electron + optional WebSocket control (V), browser-only deployment (L).
- Simulation time is decoupled from wall-clock frames: a 16 ms timer drives `updateCircuit()` (S:`:319-324`, V:`:108-122`); the solver advances `t += timeStep` (S:`:3394`, V:`:1414`, L:`:1407`) with adaptive timestep.

### 3.1 Monolithic vs refactored

- **SEVA77 is monolithic.** All solver state and stages live in the 7,124-line `S:.../client/CirSim.java`. `CircuitElm.sim` is `static CirSim` (`S:.../client/CircuitElm.java:44`), stamping through integer node indices. No `CircuitMatrix`, `CircuitLoader`, `SimulationManager`, `ElementFactory` or `SparseLU` classes exist.
- **Vactrol and LVLabs are refactored.** `CirSim.java` is a 687/649-line facade; `UIManager` owns drawing, `MouseManager` editing, `CommandManager` commands, `UndoManager` undo, `CircuitLoader` parsing, `SimulationManager` solving, `CircuitMatrix` matrix state, `ScopeManager` scopes, and `ElementFactory` (generated at GWT compile time) constructs elements. `CircuitElm.sim` is `static SimulationManager` and stamping uses `CircuitNode` objects.

## 4. Subsystem map with entry points

| Subsystem | SEVA77 anchor | Vactrol anchor | LVLabs anchor |
|---|---|---|---|
| GWT entry | `client/circuitjs1.java:49,179-192` | `client/circuitjs1.java:47,179-192` | `client/circuitjs1.java:48,174-187` |
| App/main loop | `CirSim.java:1732-1963 updateCircuit` | `UIManager.java:545-605 updateCircuit` | `UIManager.java:582+ updateCircuit` |
| Topology | `CirSim.java:2465-2524 makeNodeList`, `2677-2748 analyzeCircuit/preStampCircuit` | `SimulationManager.java:533-600,817-938` | `SimulationManager.java:529-596,813-934` |
| Matrix stamping | `CirSim.java:2771-2831 stampCircuit`, primitives `3114-3235` | `SimulationManager.java:961-1051`, primitives `1157-1277` | `SimulationManager.java:957-1047`, primitives `1153-1273` |
| Solve | `CirSim.java:6078-6196 lu_factor/lu_solve` | `SimulationManager.java:1702-1844` (dense + sparse dispatch) | `SimulationManager.java:1692-1834` |
| Nonlinear/time step | `CirSim.java:3262-3420 runCircuit` | `SimulationManager.java:1282-1448` | `SimulationManager.java:1278-1439` |
| Model state | `CirSim.java:203-287` fields | `CircuitMatrix.java:6-20`, `SimulationManager.java:49-63` | same |
| Rendering | `CirSim.java:1764,1835-1844` via `Graphics` | `UIManager.java:575,643-653` | `UIManager.java:622-692`, plus `Theme.java:25-156` |
| Editing/undo | `CirSim.java:4599+`, `5480-5511` | `MouseManager.java:328+`, `CommandManager.java`, `UndoManager.java:25-110` | `MouseManager.java:218+`, `CommandManager.java`, `UndoManager.java` (identical to Vactrol) |
| Scope sampling | `Scope.java:106-127 timeStep`, inner `ScopePlot` at `:32` | `ScopeManager.java:118-121`, `Scope.java:481-492`, `ScopePlot.java:99-121` | `ScopeManager.java:119-126`, `Scope.java:478-483`, `ScopePlot.java:99+` |
| Serialization | `CirSim.java:4159-4188 dumpCircuit`, `4372-4491 readCircuit` | `CircuitLoader.java:73-88,123-219`, `XMLSerializer.java:106-150`, `XMLDeserializer.java:177-225` | same, with `XMLSerializer.java` byte-identical to Vactrol |
| Desktop/JS bridge | `patches/CirSim.patch` + `war/js/sys.js` | `JSInterface.java:53-97`, `app/main.js`, `websocket/` | `JSInterface.java`, browser-only |
| Golden tests | `tests/` (14 dumps) | `TestManager.java:62` tolerance `1e-3`; `auto-tests/` 7 XML + golden waveforms | `auto-tests/` 7 XML (harness removed) |

Rendering is Canvas 2D throughout: `Graphics` wraps `Context2d`; high-DPI uses `devicePixelRatio()` and a canvas transform (S:`:333-389,1834-1835`; V:`:389-420,643-644`; L:`:384-427,679-680`). A 16 ms timer plus coalesced `repaint()` is the frame cadence. For a Rust/Python port, the clean snapshot tap is post-solve `onTimeStep()`: S:`:3397-3400` or V:`:1424` → `ScopeManager.timeStep()` → `ScopePlot.java:102` (`elm.getScopeValue`) and `:111-119` (buffer commit).

## 5. Solver walkthroughs

### 5.1 Voltage source + resistor (linear)

Anchors in Vactrol, with LVLabs offsets in parentheses:

1. **Load/construct:** `CircuitLoader.java:123-219` parses a dump token (`:144`, points/flags `:194-198`) and calls `app.createCe(...)` at `:200` → `CirSim.createCe` `:612-620` → `dumpTypeMap.get(tint)` `:616` → generated `factory.create` `:619`. Dump ids: `'v'` = `VoltageElm.java:92`; `'r'` = `ResistorElm.java:35`.
2. **Nodes:** `SimulationManager.makeNodeList()` `:533-600` (L:`:529-596`) assigns `CircuitNode`s; `analyzeCircuit` `:817-831` sets `needsStamp`; `preStampCircuit` `:834-938` builds voltage-source rows and sets `matrices[i].size = nodeCount + vsPerMatrix[i]` (`:925`).
3. **Stamp:** `stampCircuit()` `:961-1051` (L:`:957-1047`) calls `ce.stamp()` at `:991`. `ResistorElm.stamp()` `:112-114` → `SimulationManager.stampResistor` `:1183-1194` (g = 1/R, four matrix entries). `VoltageElm.stamp()` `:149-157` → `stampVoltageSource` `:1157-1163` (MNA ±1 plus RHS). Originals are saved (`:998-1009`) and linear matrices factored once (`:1022-1030`).
4. **Iteration/solve:** `runCircuit()` `:1282-1448`; non-linear branch is skipped for a linear network (`:1385-1386`); `lu_solve` `:1380` dispatches to dense/sparse `:1797-1804`; `applySolvedRightSide` `:1451-1471` writes `nodeVoltages` and source currents.
5. **Time/measurement:** `t += timeStep` `:1414`, `stepFinished()` `:1420`, `app.onTimeStep()` `:1424` → scope sample.

SEVA77 performs the same stages inside `CirSim.java`: `updateCircuit` `:1732-1963`, `makeNodeList` `:2465-2524` (integer node indices at `:2489,2501,2515`), `stampCircuit` `:2771-2831`, `runCircuit` `:3262-3420`, `lu_factor` `:6078-6156`, `lu_solve` `:6161-6196`, `t +=` `:3394`, direct scope fan-out `:3397-3400`.

### 5.2 Nonlinear diode / BJT

Same anchors, with the non-linear path active:

- `DiodeElm.java:88` dump `'d'`; `stamp()` `:166-175` → `Diode.stamp()` `:133-138` calls `sim.stampNonLinear()` (`SimulationManager.java:1276-1277`). `doStep()` `:176-178` → `Diode.java:140+`: step limiting `:88-126`, convergence test `|ΔV| > 0.01` at `:142-143`, `gmin` ramp after 100 subiterations at `:149-155`, conductance stamping `:161-181`.
- `TransistorElm.java:104` dump `'t'`, `nonLinear() = true` at `:83`; `stamp()` `:369-373`; `doStep()` `:374-401` tests `|ΔVbe|`/`|ΔVbc| > 0.01` and ramps per-transistor `localSubIters`.
- `runCircuit` restores the matrix and RHS each subiteration (`:1332-1341`), refactors while `!converged` (`:1372-1379`), breaks on convergence (`:1387-1388`), and on failure halves `timeStep` (`:1394`), restores `lastNodeVoltages` (`:1404`) and re-stamps (`:1405`). NaN RHS also marks non-convergence (`:1455-1457`).
- SEVA77 equivalents: `Diode.java:142,149-156`; `CirSim.java:3308` resets `converged`, `:3314-3318` restores matrix, `:3364-3379` halves timestep.

Observed numerical parameters shared across snapshots:

| Parameter | Value / rule | Anchor |
|---|---|---|
| `maxTimeStep` / `minTimeStep` defaults | `5e-6` / `50e-12` | S:`CirSim.java:4384-4385`, V/L:`CircuitLoader.java:49-50` |
| Subiterations | `(adjustTimeStep && timeStep/2 > minTimeStep) ? 100 : 5000` | S:`:3306`, V:`:1328`, L:`:1321` |
| Timestep doubling | after 3 good iterations | S:`:3294-3300`, V:`:1314-1320`, L:`:1310-1316` |
| Speed → iterations | `.1 * exp((speedBar-61)/24)` | S:`:3237-3244`, V:`:320-327`, L:`:297-304` |
| Diode convergence | `|ΔV| > 0.01`, `gmin` ramp `/3000` | `Diode.java:142,149-156` (all three) |
| BJT `gmin` | `1e-12`, ramp `/300` | S:`TransistorElm.java:241-246`; V:`:391-400`; L:`:354-363` |
| Diode thermal voltage | `vt = 0.025865` (27 °C) | S:`Diode.java:63` |
| Floating-node resistor | `1e8` to ground | S:`CirSim.java:2592-2598` |
| Dense LU zero pivot | reject (commented `1e-18` workaround) | S:`:6143-6146`, V:`:1781-1784`, L:`:1774-1777` |

### 5.3 Observable differences that affect results

- **Solver choice:** SEVA77 has dense Crout LU only. V/L add `SOLVER_AUTO/DENSE/SPARSE` (`SimulationManager.java:42-47`), `SPARSE_THRESHOLD = 150` (`:46`), EJML-derived sparse CSC LU (`matrix/SparseLU.java`, byte-identical in V and L), and persist the choice as XML attribute `st` (`XMLSerializer.java:129-130`).
- **BJT gmin trigger:** SEVA77 uses global `sim.subIterations > 100`; V/L use a per-transistor `localSubIters > 100` counter. Same constants, different convergence trajectory.
- **Voltage source internals:** `VoltageElm.java` grew 374 → 688/682 lines; V/L stamp `internalResistance` (V:`:156`), absent in S:`:103-108`.
- **Vactrol-only physics:** `VactrolModel.java:41-51` advances conductance with an exact first-order update (stable when timestep changes); `VactrolElm.java:285-303` stamps diode + conductance. BJT avalanche adds stateful, hysteretic branches and synthetic noise (`BjtAvalancheModel.java:19-112`, `TransistorElm.java:306-367,560-651,892-899`); `AudioOutputElm.java:230-247` forces `timeStep = 1/sampleRate`, changing global simulation cadence when present.
- **`getIterCount()` in SEVA77** carries the comment "IES - remove interaction" (`S:.../client/CirSim.java:3238`); no patch touches solver code, so this is a visible source-level deviation but not evidence of changed numerics.

## 6. Component registration and dump identifiers

- **SEVA77:** inline `switch (tint)` in `CirSim.createCe()` `6230-6355`; e.g. `'c'` `:6242`, `'d'` `:6243`, `'r'` `:6252`, `'v'` `:6255`, `403` Scope `:6325`. Palette aliases are built in `constructElement()` `:6357-6637`; menus register classes via `getClassCheckItem(..., "ClassName")` `:1343-1524`. Text dump format at `CircuitElm.java:223-227`; options header at `CirSim.java:4144-4157`.
- **Vactrol/LVLabs:** `CirSim.register()` V:`:576-610` / L:`:539-573` populates `dumpTypeMap` and `xmlDumpTypeMap` from `elm.getDumpType()`; `createCe()` V:`:612-620` / L:`:575-583` resolves via `factory.create()`; the factory is generated from all `CircuitElm` subtypes by `rebind/ElementFactoryGenerator.java:93-129`, wired in `circuitjs1.gwt.xml:7-9`. Registration is triggered from `UIManager.getClassCheckItem()` (V:`:1382-1417`, L:`:1363-1394`).
- **Common ids:** `'r'` resistor, `'c'` capacitor, `'l'` inductor, `'v'` voltage source, `'d'` diode, `'t'` transistor, `'g'` ground, `'w'` wire, `403` scope; `DCVoltageElm.java:24-25` shares `'v'` via `getDumpClass() → VoltageElm.class`. Vactrol's vactrol uses hard-coded id `391` (`VactrolElm.java:24`, consistent with `examples/vactrol-demo.txt:5`).
- **Serialization formats:** legacy text dumps (`$ options`, one element per line, optional `?ctz=` LZ-String URL compression via `ExportAsUrlDialog.java:89-90`) are understood in all three; V/L additionally use XML (`XMLSerializer.java:106-150`; `CircuitLoader.java:73-81` dispatches on `<`).

## 7. Fork-by-fork comparison

### SEVA77 — desktop integration and usability

- 17 patch files are the canonical customization mechanism (`patches/README.md:1-5`: `cat patches/*.patch | patch -p0`) applied after replacing upstream Java sources. `patches/CirSim.patch` adds NW.js `executeJS`, screen scaling, `MOD_*` preferences in localStorage, native save/open via `nw.require('fs')`, and window-title/unsaved-change tracking. `patches/Toolbar.patch:35-44` adds New/Open/Save/Save As buttons; `patches/EditDialog.patch:162-166` adds color pickers; `patches/EditOptions.patch` marks five fields as colors.
- Packaging: Maven `pom.xml:4-7` `circuitjs1mod 1.3.2`, `scripts/dev_n_build.js:9,207-264` builds NW.js bundles, `Inno Setup/setup_x64.iss` and `setup_x32.iss` produce installers.
- **Observed:** no patch changes stamping, `lu_factor`, convergence or timestep logic. The solver is upstream-style Falstad with desktop UI/file I/O modifications. `S:README.md:9` warns that many components are idealized.

### Vactrol — models and audio

- Only these Java files carry 2026 Nerding and Hoarding / SPDX GPL-3.0-or-later markers: `VactrolElm`, `VactrolModel`, `BjtAvalancheModel`, `AudioOutputElm`, `Menus`, `TransistorElm`, `TransistorModel`. Everything else retains Falstad/Sharp GPL-2.0-or-later headers.
- Vactrol transfer curve: `Rtarget = clamp(30 kΩ × (I_LED/1 mA)^-0.8, 1.5 kΩ, 10 MΩ)`; checkpoints 0 mA → 10 MΩ, 1 mA → 30 kΩ, 10 mA → 4.75 kΩ, 40 mA → 1.57 kΩ; rise 2.5 ms, decay 8.403 ms (`VACTROL_DEVELOPMENT.md:23-56`; `VactrolModel.java:19-51`). Conductance state is serialized (`dr,r1,ex,mr,tn,tf,cg` at `VactrolElm.java:143-168`).
- BJT avalanche: softplus reverse current, snapback branch/hysteresis and noise (`BJT_AVALANCHE_DEVELOPMENT.md:11-106`; `TransistorModel.java:150-160,206-262` presets for 2N2222/2N2222A/2N3904).
- Audio output: 60 s capture/export with sample-rate-dependent timestep (`AudioOutputElm.java:23-24,163-247,279-365`).
- Generic upstream elements such as `GyratorElm`, `NortonAmpElm`, `AnalogMuxElm` and bus logic are **not** fork additions (upstream headers present).
- Desktop: sandboxed Electron (`app/main.js:76-113` with `contextIsolation: true`, `sandbox: true`, `nodeIntegration: false`), Windows packaging script, and a Python WebSocket control channel (`websocket/circuitws_server.py`, commands listed in `websocket/README.md:58-75`). **Observed follow-up:** `README.md:233-238` still documents Electron 9.3.2 while `app/package.json:29` pins 43.4.0.

### LVLabs — codebase simplification

- Removes Electron/websocket/test-harness/dev-server/IDE metadata (`L:README.md:21-27`); retains the full element library (151 `*Elm.java`, dump ids through 436), 374 example circuits, 14 locales, scopes and composite models.
- `CirSim.java` is only 649 lines because logic moved into managers, not because features were deleted. `UIManager.java` (1,554 lines), `MouseManager.java` and 12 `Scope*.java` files are all present.
- `Theme.java:25-156` adds CSS-variable-driven theming; `SubcircuitBar.java:23-32` still hard-codes light colors (inferred: partial theming adaptation).
- The 7 XML `auto-tests/` fixture files are present but orphaned — no code references them (the `TestManager` harness is absent).
- **Conclusion:** LVLabs is leaner in *repository and build infrastructure*, and slightly leaner in runtime class count because it drops Vactrol-specific models. It is **not** a leaner simulator core: the MNA solver, component set, scopes and UI are essentially the same refactored upstream 4.1.x code. Do not treat "fewer files" as "simpler solver".

## 8. Recommended donors and baseline

“Baseline” means the snapshot to read first when porting a subsystem; it does not authorize copying. Apply the [import gate](../PROVENANCE.md) before reusing any source.

| Subsystem | Baseline | Rationale / evidence | Alternatives |
|---|---|---|---|
| Topology + matrix state | Vactrol / LVLabs | Explicit `CircuitNode`/`CircuitMatrix` types (`CircuitMatrix.java` byte-identical in V/L); maps cleanly to Rust structs. | SEVA77 integer-node approach is simpler but monolithic (`CirSim.java:2465-2524`). |
| Solver core + time stepping | LVLabs | `SimulationManager.java:813-1047,1278-1439,1692-1834` is the smallest refactored implementation; no fork-specific model state. | Vactrol is ~9 lines offset with changed BJT gmin; SEVA77 is dense-only and embedded in UI code. |
| Component physics | LVLabs for generic models | `Diode.java`, `TransistorElm.java`, `CapacitorElm.java:167-169` (trapezoidal vs backward Euler) are upstream-style. | Vactrol only for vactrol/BJT-avalanche/audio features (Task 10). |
| Registration + serialization | Vactrol | Generated `ElementFactory` + dual text/XML loaders give an explicit registration contract; `XMLSerializer.java` is byte-identical in V/L. | LVLabs loader differs by one line; SEVA77 inline switch + text-only format is useful for legacy interop. |
| Rendering + UI + editing | LVLabs | Clean `CirSim`/`UIManager` split plus `Theme.java`; no Vactrol-only drag/drop to untangle. | Vactrol `MouseManager.java:208-304` adds drag-and-drop creation; SEVA77 has native file/window integration. |
| Scope / measurement stream | Vactrol | Decomposed `ScopeManager`/`ScopePlot`/`ScopeTrigger`/`ScopeSerializer`; helpers byte-identical to LVLabs. | LVLabs equivalent; SEVA77 single `Scope.java` still usable as a compact reference. |
| Desktop/host integration | SEVA77 for NW.js | `Inno Setup/`, `patches/`, `scripts/dev_n_build.js` are the only NW.js desktop recipe. | Vactrol `app/` + `websocket/` for sandboxed Electron and a Python control channel; LVLabs for browser-only. |
| Validation harness + fixtures | Vactrol | `TestManager.java:62` (`TOLERANCE = 1e-3`) plus 7 golden-waveform XML fixtures. | LVLabs keeps the fixtures but not the harness; SEVA77 keeps 14 text dumps. |

Baseline selection rationale: avoid merging forks wholesale. LVLabs is the closest thing to a clean upstream 4.1.x runtime with the refactor already done; Vactrol contributes specific physics and a validation harness; SEVA77 contributes desktop packaging only. For the Rust engine, the refactored `SimulationManager` flow is the natural boundary — it already separates topology, stamping, iteration, solve and time stepping, matching Task 02's ownership split.

Open questions to resolve in Task 02/03:

1. Dense vs sparse solver placement: reproduce `SPARSE_THRESHOLD = 150` in Rust, or start dense-only and treat sparse as an optimization?
2. Which nonlinear convergence rule to port: global `subIterations` (S) or per-element `localSubIters` (V/L)? The latter is likely the baseline, but it changes results.
3. Whether to preserve both text dumps and XML from day one, or implement the text dump first and treat XML as an import extension.
4. Scope snapshot cadence: sample at `ScopePlot.java:102` (raw `getScopeValue`) or after the AC-coupling filter at `:111-119`.

## 9. Candidate validation fixtures

| Behavior | Fixtures | Notes |
|---|---|---|
| Resistor divider | `S:.../public/circuits/voltdivide.txt`; `V/L:.../public/circuits/voltdivide.txt` | Same circuit in text and XML across forks; exact analytic ratios. |
| RC transient | `V/L:auto-tests/capac.txt` | Includes `c,r,v,w` and golden `<scopedata>`; `len=0.042365`. |
| RLC / switching | `V/L:auto-tests/lrc.txt`, `lrcswitch.txt` | Golden waveforms; `lrc.txt:25-31` `<test len="0.025015...">`. |
| Nonlinear diode | `V/L:auto-tests/fullrect-sc.txt`, `V:tests/fullrect.txt` | Four diodes, composite subcircuit, switch events. |
| Nonlinear OTA | `V/L:auto-tests/ota-vca.txt` | Nonlinear golden waveform, `len=0.058145`. |
| Digital | `V/L:auto-tests/td4.txt`, `S:tests/tflipflop.txt` | Chip-level timing. |
| Vactrol / avalanche | `V:examples/vactrol-demo.txt`, `V:examples/bjt-avalanche-noise.txt`, `V:model-tests/*`, `V:.../public/circuits/synth-mod/*` | Vactrol-only; useful for Task 10, not baseline validation. |

SEVA77 additionally ships 342 circuits indexed by `public/setuplist.txt`; Vactrol 373 and LVLabs 374. These are broad behavior corpora but lack golden waveforms, unlike `auto-tests/`.

## 10. Licensing and provenance follow-ups

- No source was imported in Task 01. The [PROVENANCE.md](../PROVENANCE.md) import gate still applies before any port.
- SEVA77: GPL-2.0-or-later; `war/about.html:44-79` and `war/canvas2svg.js:5-11` contain third-party credits (LZString, canvas2svg MIT, font licenses); `LicenseDialog.patch:5` adds a Usevalad Khatkevich copyright line. No consolidated NOTICE file.
- Vactrol: combined work GPL-3.0-or-later; upstream GPL-2.0-or-later preserved; fork-authored files carry 2026 SPDX headers; EJML-derived `matrix/*.java:1-18` are Apache-2.0; `THIRD_PARTY_NOTICES.md:3-13` lists GWT, Electron, LZ-String, canvas2svg and fonts. `RELEASE_NOTES.md:15` notes the beta is not Authenticode-signed; `MODIFICATIONS.md:31` claims CI/provenance but no `.github/` directory exists in the snapshot.
- LVLabs: GPL-2.0-or-later, but `build.gradle:4` retains a GPLv3 header from E3V3A — reconcile before reuse. `war/lz-string.min.js` has no visible license header; `war/avr8js/` bundles third-party code with no local license file. `war/about.html:22` still credits removed WebSocket support.
- Per-file headers consistently credit Paul Falstad and Iain Sharp; language ports are still derivative works under the import gate.

## 11. Acceptance checklist

- [x] All three snapshots accounted for (file counts, normalized source hashes, versions, licenses, build/runtime).
- [x] Subsystem map with entry points and line ranges, not just README claims.
- [x] Component registration and dump identifiers located with line numbers.
- [x] One linear and one nonlinear circuit traced through topology → stamping → solve → timestep → measurement.
- [x] SEVA77 desktop, Vactrol model/audio and LVLabs removals compared; LVLabs found leaner in infrastructure, not core runtime.
- [x] Recommended donor per subsystem with alternatives and evidence.
- [x] Licensing follow-ups recorded.
- [x] Candidate fixtures listed for divider, RC, nonlinear and digital validation.
- [x] No implementation or reference modifications.
