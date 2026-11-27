# Electrow release notes

## 0.2.0 — 2026-11-27

MVP source release. Rust worker + Python local HTTP server + vanilla-JS browser schematic editor. No third-party runtime packages, no NW.js/Electron, no bundled assets.

### Included

- Dense modified-nodal-analysis solver: wires, ground, resistors, independent voltage/current sources, switches, capacitors, inductors, Shockley diodes, simplified BJT/MOSFET and a four-terminal Vactrol optocoupler.
- Versioned Rust/Python protocol, revision-safe edits, subscriptions, bounded diagnostics, session/reconnect handling and a 16 ms Python step driver.
- Browser editor: Canvas 2D grid/zoom/pan, palette, selection, drag, wire tool, undo/redo, voltage/current display, animated current flow and a bounded 4096-sample scope.
- Electrow JSON, Falstad text and reference XML import; circuit save/load/list endpoints.
- Fixture corpus, Rust/Python/browser tests, CI workflow and performance harness.
- Optional trusted-LAN hosting with `--allow-lan` or a non-loopback bind. Loopback remains the default.

### Known limitations

- Idealized device models are not SPICE-equivalent. No saturation/Early effects, body diodes, controlled sources, op-amps, digital gates, diode breakdown or avalanche/noise.
- AC waveform sources and true rectifier fixtures are not supported; Falstad dumps with unsupported records report them instead of substituting models.
- Browser geometry is display-only in the circuit document; imported documents without geometry are auto-laid-out.
- LAN mode disables Host/Origin checks and is for trusted development networks only.
- Request bodies are capped at 1 MiB; worker responses at 4 MiB.
- Dense LU remains the bottleneck above roughly 100 unknowns; sparse solving is not implemented.

### Distribution and license

- Original code: GPL-3.0-or-later; see `LICENSE`.
- Behavior-derived Vactrol model and browser workflow notices: `THIRD_PARTY_NOTICES.md` and `PROVENANCE.md`.
- Reference repositories in `ref_SEVA77/`, `ref_Vactrol/` and `ref_LVLabs/` are review-only and are excluded from release archives.
- `Docs/release.md` records build steps, dependency inventory, verification evidence and the corresponding-source delivery plan.
