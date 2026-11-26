# Electrow implementation tasks

Goal: preserve the interactive Falstad experience while moving circuit simulation into Rust, hosting a local web UI through Python, and keeping browser JavaScript simple.

## Execution order

| Task | Scope | Depends on |
|---|---|---|
| [01](01-reference-analysis.md) | Compare reference source snapshots | — |
| [02](02-architecture.md) | Define boundaries and contracts | 01 |
| [03](03-rust-solver.md) | Port numerical simulation core | 02 |
| [04](04-component-models.md) | Port component models | 03 |
| [05](05-circuit-format.md) | Circuit data and Falstad import/export | 02, 04 |
| [06](06-python-backend.md) | Local server and Rust integration | 02, 03, 05 |
| [07](07-web-ui.md) | Interactive browser schematic | 05, 06 |
| [08](08-validation.md) | Numerical and integration verification | Start with 03; continue throughout |
| [09](09-performance.md) | Profile and optimize | 08 |
| [10](10-fork-features.md) | Select and integrate additional features | 04, 07, 08 |
| [11](11-release.md) | Packaging and release readiness | 06–10 |

## Rules for every task

- Reference folders at repository root: `ref_SEVA77/`, `ref_Vactrol/`, `ref_LVLabs/`. Treat them as read-only. If absent, report the blocker rather than guessing source contents.
- Use only these three code donors. Work from their current checked-out snapshots; record exact revision or content hashes. Consult history only for selected licensing/provenance exceptions.
- Apply the import gate in [PROVENANCE.md](../PROVENANCE.md) before copying or translating code; record notices in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). A language port is not automatically independent code.
- Keep each implementation task small and testable; record source paths, design choices, commands, actual test results and unresolved issues. Do not mark a task complete without evidence.
- No performance claims without measurement. Keep numerical correctness ahead of optimization.
- Do not commit reference copies or package them automatically without verifying applicable redistribution obligations.
