# Fix 11 — Validate XML import options consistently

Status: complete (2026-11-26). XML importer now uses the same shared option validator; absent `vr` uses the default instead of 0. Added `xml_rejects_non_finite_and_inverted_options`; 84 tests pass.
Severity: medium (invalid simulation settings)

## Problem
`xml_import.rs` uses `mts` unvalidated (NaN/inf/negative accepted) and only checks `vr > 0.0`, so `vr="inf"` is accepted because `f64::from_str` parses `inf`. This disagrees with both the text importer's finite check and `validate.rs`'s finite + bound check.

## Evidence
- `engine/src/xml_import.rs:330-342` — option parsing.
- `engine/src/falstad/parser.rs:89-94` — finite check reference.
- `engine/src/circuit_doc/validate.rs:99-106` — finite + `<= 1e6` reference.

## Fix
Share a single option-validation helper between the text and XML importers (or call `circuit_doc::validate` on the produced document) so both paths enforce identical finite/range rules. Report bad values with the XML line/attribute context.

## Acceptance
- `<options mts="NaN">`, `mts="-1"`, `vr="inf"` are rejected consistently with the text importer.
- Existing XML fixtures still pass; text/XML comparison test still passes.
- `cargo test --workspace` passes.
