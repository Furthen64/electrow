# Task 07 — Lightweight browser schematic UI

Status: pending. Depends on 05 and 06.

Use HTML/CSS and modular vanilla JavaScript. Select Canvas 2D or SVG after a small rendering/interaction spike; favor simple dependencies and a clear rendering layer. Do not port the entire reference UI or desktop runtime blindly.

MVP: toolbar, component palette, schematic viewport, grid, zoom/pan, selection, component placement/movement/deletion, wire creation with explicit junctions, parameter editing, import/export, run/pause/reset/step and status diagnostics. Support undo/redo for document edits with revision-safe backend synchronization. Display voltage/current with units, polarity and animated current flow based on actual solver snapshots. Add oscilloscope plots with bounded sample buffers and time axes.

Separate document/editor state, transport, renderer, symbol registry and scope widgets. Use requestAnimationFrame for display only; interpolate animation visually without modifying physics. Keep reliable edit acknowledgments separate from replaceable snapshots. Handle pointer capture, coordinate transforms, keyboard shortcuts, high-DPI displays and reconnect state.

Acceptance: build a resistor divider and RC circuit interactively, run simulation, observe correct measurements and scope trace, edit parameters, save/reload and import a supported Falstad fixture. No frontend solver or heavy JS framework required.
