# Release readiness

## 0.2.0 — 2026-11-27

This repository is a source release with an optional release worker binary. It does not ship NW.js, Electron, pip packages, npm packages, fonts, icons or bundled reference repositories.

## Supported targets

| Target | Status | Notes |
|---|---|---|
| Linux x86_64 (glibc) | Tested | Kernel 6.0, rustc 1.93.1, Python 3.14.4, Node 24.20.0, Chrome 156 |
| Windows 10/11 x64 | Scripts provided, not executed on the Linux build host | `checkreqs.ps1`, `winbuild.ps1`, `test.ps1`, `winlaunch.ps1` |
| macOS | Not officially supported | POSIX scripts may work but are untested |

Runtime requirements: Rust-built worker (no crates outside `std`), Python 3.12+ standard library. Node 22+ and Chrome are needed only for the browser harness. `Cargo.lock` contains only `electrow-engine`; there are no pinned third-party runtime versions to manage. CI pins Python 3.12 and Node 22.

## Build and launch

```sh
./checkreqs.sh
cargo test --workspace --offline
cargo build --workspace --release --offline
python -m backend --host 127.0.0.1 --port 8000 --open
```

`--open` uses Python's `webbrowser` module and opens loopback even when the server binds `0.0.0.0`. `./launch.sh --open` and `winlaunch.ps1 -Open` pass this through. A non-loopback bind or `--allow-lan` enables trusted-LAN mode and prints a warning; the default remains loopback-only with Host/Origin checks.

Port conflicts exit with code 2 and a clear message. `Ctrl+C` shuts down the HTTP server and Rust worker cleanly. Builds and tests work offline because there are no external dependencies.

## Packaging decision

Source-first packaging: `release.sh` verifies the build, runs all regressions, produces a source tarball and release worker binary under `Temp/release-<version>/`, extracts the tarball into `Temp/clean-env-<version>/` and reruns offline smoke/API tests there. Checksums are written to `SHA256SUMS`.

No desktop shell is packaged: the UI is a local web page served over HTTP. No reference folders, `Temp/`, `circuits/`, `FutureSimulation/` research images or bytecode are included in the archive.

## Dependency and license inventory

| Component | Source | License / obligation |
|---|---|---|
| Rust engine | `engine/` | GPL-3.0-or-later, original code; `std` only |
| Cargo.lock | repository | Only `electrow-engine`, no external crates |
| Python backend | `backend/` | GPL-3.0-or-later, stdlib only |
| Browser UI | `web/`, `web/js/` | GPL-3.0-or-later, vanilla ES modules, no npm packages |
| Vactrol behavior | `ref_Vactrol`, GPL-3.0-or-later, 2026 Nerding and Hoarding | Behavior-derived, no source copied; notice in `THIRD_PARTY_NOTICES.md` |
| Browser workflow pattern | `ref_SEVA77`, GPL-2.0-or-later | Interaction pattern reimplemented, no source copied |
| LICENSE / notices | `LICENSE`, `PROVENANCE.md`, `THIRD_PARTY_NOTICES.md` | Included in source archive |

No GPLv3-only material was adopted. The Vactrol donor is GPL-3.0-or-later, compatible with Electrow's original-code policy. No fonts, icons, images or example circuits from donors are packaged. Unresolved contributor code such as `LDRElm.java` remains excluded.

## Corresponding-source plan

Every binary artifact is produced alongside `electrow-<version>-source.tar.gz`, which contains the complete source, build scripts, tests, docs, `LICENSE`, `PROVENANCE.md` and `THIRD_PARTY_NOTICES.md`. If a binary is distributed separately, the distributor must provide a written offer valid for at least three years for the complete corresponding source, or ship the archive with it.

## Verification evidence

`./release.sh` on the tested Linux host completed with:

- offline Rust tests: 93 unit + 12 integration passed, 0 failed, 1 ignored benchmark;
- Python smoke: SMOKE OK; Python API: 5 passed;
- browser harness: 15/15 checks passed;
- clean-environment extraction: offline Rust tests/build, smoke and API tests passed;
- archive: 124 files, no reference/Temp/circuits/FutureSimulation paths or binary assets, checksums recorded in `Temp/release-0.2.0/SHA256SUMS`;
- port conflict exits with code 2 and a clear `Address already in use` message; SIGINT exits with code 0.

Known limitations and unsupported components are listed in [RELEASE_NOTES.md](../RELEASE_NOTES.md).
