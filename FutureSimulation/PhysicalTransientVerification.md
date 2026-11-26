# Future simulation: physical transient verification

**Status:** Future research proposal — not an active implementation task.

**Prerequisite:** The Rust circuit engine, Python backend, browser UI and basic numerical regression tests must be operational. Do not interrupt the current implementation roadmap to build this feature.

## Vision

Electrow should eventually be able to compare simulated transient responses with real oscilloscope measurements of physical circuits. The goal is to learn where ideal circuit models are sufficient, where physical effects matter, and whether adding those effects actually improves predictive accuracy.

This is an experimental learning project, not an assumption that more elaborate models are automatically more accurate. Start with the simplest possible circuit and work out the measurement, modelling and comparison workflow before introducing additional complexity.

## 1. First experiment: a resistive circuit

Start with an isolated, low-voltage battery, a switch and a measured resistor (for example, 1 kΩ with a 3–5 V battery). Connect the resistor between the switched supply and battery negative. Connect the oscilloscope probe ground only to battery negative and measure the switched resistor voltage.

Record the actual battery voltage under load, resistor value, wiring arrangement and instrument settings. Capture several switch closures, including the pre-trigger interval. Export the raw time/voltage samples to CSV if supported by the instrument.

First compare the settled voltage with an ideal simulation. The switch transition may reveal contact bounce, but do not initially attempt to fit high-frequency ringing or infer wire inductance from it. An ideal resistive step is instantaneous; the physical source, switch and instrument have finite bandwidth and dynamics.

**Success criterion:** reproduce the physical circuit in Electrow, import or externally read a measured trace, align measured and simulated time axes, and explain the principal differences. Preserve the original measurement data.

## 2. Second experiment: a measurable RC transient

Add a capacitor across the resistor/output of a suitable series-R circuit, creating a low-pass RC network: battery positive → switch → series resistor → output; capacitor from output to battery negative. For example, 10 kΩ and 100 nF give a nominal time constant of 1 ms. Ensure a repeatable discharged initial condition before each capture. Measure actual R and C where possible.

Measure the input step and capacitor voltage with two channels sharing battery negative, or capture the output with a single channel and document the input separately. Use a sampling interval much shorter than the expected time constant, adequate recording duration and consistent trigger settings.

Compare the measured output with the analytical result for an ideal step and the Rust simulation:

- Time constant: τ = R × C.
- For an initially discharged capacitor and constant step V: Vc(t) = V × (1 − exp(−t/τ)).
- At one time constant, the voltage reaches approximately 63.2% of its final value.
- The 10–90% rise time is approximately 2.2τ.

If the physical input is not an ideal step, use the recorded input as a stimulus in a later comparison. Separate source variation, component tolerance and instrument loading from numerical solver error.

**Success criterion:** obtain repeatable captures, establish a defensible uncertainty/tolerance budget, compare the analytical, simulated and measured responses, and document any unexplained discrepancy. Do not prescribe an arbitrary accuracy threshold before collecting measurements.

## 3. Third experiment: battery, switch, resistor and LED

Once the RC workflow is reliable, add a nonlinear component. Example circuit: 4.5 V battery → switch → 330 Ω current-limiting resistor → LED → battery negative. Adjust the resistor for the actual supply and LED ratings.

Measure CH1 at the switched supply and CH2 at the LED anode, both referenced to battery negative. Infer current as (CH1 − CH2)/R using the measured resistor value when its parasitic impedance is negligible and channels are suitably aligned. Record the LED part number and available datasheet characteristics.

Compare steady-state voltage/current, turn-on behaviour and repeated switch bounce captures. Use a repeatable electronic step source later if needed; do not assume mechanical switch closures are identical.

All initial experiments should remain isolated and low voltage. Do not connect an earth-referenced oscilloscope ground clip to arbitrary energized nodes or mains circuits. Verify instrument grounding and probe ratings before measurement.

## 4. Simple and Realistic modes

Future Electrow may support two modelling policies for the same schematic:

| Effect | Simple mode | Realistic mode candidate |
|---|---|---|
| Wire | Ideal connectivity | Series resistance/inductance and capacitance to relevant conductors |
| Source | Ideal voltage source | Measured internal resistance, optional dynamic behaviour |
| Switch | Ideal open/closed | Contact resistance, measured or statistical bounce |
| Capacitor | Nominal capacitance | Measured capacitance, ESR, ESL, leakage |
| Breadboard | Ideal connections | Contact resistance and inter-conductor capacitance |
| Probe | Ideal observation | Input resistance/capacitance and ground-return effects |

Realistic mode is a hypothesis to test. Start with only one additional effect at a time, compare both modes against independent measurements, and retain enhancements that improve prediction within the measurement uncertainty.

## 5. Future wire and parasitic modelling

A proposed approach is to compile the user schematic into an expanded electrical network before sending it to the same Rust numerical solver. Simple mode merges ideal connections; Realistic mode may replace selected connections with internal nodes and parasitic elements. This avoids duplicating solver algorithms.

Physical wire properties could include conductor material, cross-sectional area, physical length, temperature, insulation, return-path geometry and measured or estimated per-unit-length parameters.

- Resistance can be estimated using R = ρ × length / area.
- Inductance depends on current loop geometry and return path.
- Capacitance depends on other conductors, spacing and dielectric material; it is not simply an intrinsic capacitance of an isolated wire.
- Schematic drawing length is not physical wire length.
- Start with lumped RLC equivalents; distributed transmission-line models and mutual coupling are later topics if measurements justify them.

Allow explicitly supplied values and distinguish measured, datasheet-derived, estimated and unknown parameters. Do not silently invent physical dimensions. Keep generated parasitics inspectable and preserve mapping to the original schematic. Additional nodes and fast time constants may increase solver cost or cause numerical stiffness.

## 6. Waveform comparison workflow

Initially use manual CSV export and an offline comparison script or notebook. Only integrate the workflow into the Python backend and WebUI after the process is understood. Direct USB/LAN instrument communication (SCPI/PyVISA, where supported) is optional future work.

For each capture, retain:

- Experiment ID, schematic and photograph or wiring diagram.
- Component identifiers, actual measured values, physical wire lengths and setup notes.
- Scope model, probe attenuation, input impedance, bandwidth limit, coupling, sample interval, trigger and channel settings.
- Raw unmodified waveform files and channel mapping.
- Simulation version/commit, model parameters, initial conditions, time-step settings and output.
- Time alignment method, comparison script version, uncertainties and conclusions.

Overlay raw measured, Simple-mode and Realistic-mode waveforms with explicit units and time alignment. Preserve raw data when resampling or filtering. Report steady-state error, rise time, time constant, overshoot, settling time, ringing frequency or waveform RMSE only where relevant and reliably measurable.

Probe loading, acquisition bandwidth, quantization, noise, triggering, component variation and measurement repeatability can limit the comparison. If modelling instrument loading, add the electrical probe model separately from any acquisition/filtering model.

## 7. Learning and validation principles

- Change one variable at a time and repeat measurements.
- Begin with analytical circuits; compare with Falstad where useful, but neither Falstad nor physical data alone proves numerical correctness.
- Distinguish verification of the numerical implementation from validation of physical models.
- Prefer measured component values over nominal values when available.
- Fit unknown parameters only where identifiable; keep separate captures or circuit configurations for validation rather than scoring the same data used for fitting.
- Record negative results, uncertainty and failed assumptions. Do not add parasitics solely to make one trace look better.
- Promote stable, documented captures to regression fixtures only after the measurement workflow is reliable.

## 8. Deferred milestones

1. Manual low-voltage resistive experiment and documented capture.
2. RC transient with analytical and simulated comparison.
3. LED switching and nonlinear model comparison.
4. Repeatable waveform import and comparison tooling.
5. One independently justified nonideal model (e.g. battery internal resistance or capacitor ESR).
6. Simple/Realistic modelling policy and inspectable network expansion.
7. Wire/breadboard/probe parasitic modelling, validated incrementally.
8. Optional instrument automation, parameter identification and regression corpus.

The first milestone does not require a Realistic mode, automated oscilloscope control, a physical component library or a dedicated Verification tab. Those features should emerge from practical experience.

## Open decisions

Actual oscilloscope model/export format, probe specifications, available components, appropriate sampling settings, acceptance tolerances and eventual UI design remain unknown. Determine these when this research track starts. This document is deliberately a future proposal, not a modification of the current solver implementation or task priorities.
