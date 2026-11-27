# Task 10 — Select useful fork-specific functionality

Status: complete (2026-11-27). Depends on 04, 07, 08. Delivered a prioritized feature matrix (`Docs/fork-features.md`) with adopt/defer/reject decisions and donor evidence; adopted and implemented the Vactrol four-terminal optocoupler in Rust with transfer/update unit tests and fixture `10-vactrol.json`, browser keyboard/file workflow (Ctrl+S/O/N plus unsaved badge) and simplified BJT presets with browser-harness checks. Provenance recorded in `PROVENANCE.md` and `THIRD_PARTY_NOTICES.md`; no donor source text copied. Evidence: `cargo test --workspace` 105 passed / 0 failed / 1 ignored; `python tests/smoke.py` SMOKE OK; `python tests/test_api.py` 5 passed; `./tests/run_browser_ui.sh` 15/15. Avalanche/noise and audio export remain deferred with rationale; LVLabs infrastructure and unresolved contributor code rejected.

Use task 01 comparison to select features based on user value, correctness, maintenance cost and license eligibility. Do not assume README claims are implemented or that the latest fork is best.

Investigate Vactrol native optocoupler models, reverse-avalanche/zener behavior, transistor noise/presets and audio export as candidate features; verify actual source and numerical assumptions. For audio, define sampling rate, simulation-time resampling, clipping and WAV export tests. For noise, document reproducible seeding and physical limitations. For SEVA77, identify desktop/offline usability and keyboard/file workflow improvements that fit a browser UI. For LVLabs, adopt genuinely useful simplifications rather than blindly copying removed infrastructure.

Create one subtask per chosen feature with donor paths, import gate evidence, Rust/Python/JS ownership, model or UI tests and acceptance criteria. Defer unclear contributor code (including identified LDRElm.java and other exceptions) until verified or independently replaced.

Acceptance: prioritized feature matrix with adopt/defer/reject decisions; selected features implemented in isolated changes with regression tests and provenance records.
