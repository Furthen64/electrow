# Electrow

Electrow is a planned open-source circuit simulator with a web-based interface, built in Rust and Python and inspired by Falstad/CircuitJS. Its goal is to deliver a faster, more capable and maintainable circuit simulation application for local use.

## Project status

Early development. Reference implementations are being evaluated before code integration.

## Scaffold quick start

Check the local toolchain (cargo, rustc, Python 3.12+):

```powershell
./checkreqs.ps1
```

Build and test the Rust engine worker, then launch the Python web server:

```powershell
cargo test --workspace
cargo build --workspace
python tests/smoke.py
python -m backend
```

Open `http://127.0.0.1:8000/`. Python talks to the Rust worker over a versioned newline-delimited JSON protocol; set `ELECTROW_ENGINE` to override the worker path. See [Docs/architecture.md](Docs/architecture.md) and [Docs/interface-schema.json](Docs/interface-schema.json). The engine is a contract placeholder, not a simulator yet.

## PowerShell scripts

| Script | Purpose |
|---|---|
| `./checkreqs.ps1` | Check cargo, rustc and Python 3.12+ |
| `./build.ps1` | Check requirements, build Rust, syntax-check Python and JS |
| `./test.ps1` | Build, run Rust unit tests and the real-worker HTTP smoke test |
| `./run.ps1` | Build and launch the local server; `-NoBuild` skips the build, `-Port 8001` changes the port |

## License

Electrow's original code is licensed under the GNU General Public License, version 3 or (at your option) any later version.

SPDX-License-Identifier: `GPL-3.0-or-later`

See [LICENSE](LICENSE) for the full GNU GPL version 3 text. Imported material must retain applicable upstream copyright notices and licensing terms.

See [REFERENCES.md](REFERENCES.md) for candidate upstream repositories, [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party notices, and [PROVENANCE.md](PROVENANCE.md) for code import records.

Electrow is an independent project and is not an official Falstad release.
