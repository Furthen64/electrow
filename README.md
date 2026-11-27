# Electrow

Electrow is a planned open-source circuit simulator with a web-based interface, built in Rust and Python and inspired by Falstad/CircuitJS. Its goal is to deliver a faster, more capable and maintainable circuit simulation application for local use.

## Project status

All implementation tasks are complete. See [RELEASE_NOTES.md](RELEASE_NOTES.md) and [Docs/release.md](Docs/release.md) for targets, packaging and known limitations.

## Run locally

Requirements: Rust (cargo/rustc) and Python 3.12+. Check the toolchain with `./checkreqs.sh` (POSIX) or `./checkreqs.ps1` (Windows), then:

```sh
cargo test --workspace
cargo build --workspace
python tests/smoke.py
python -m backend --host 127.0.0.1 --port 8000 --open
```

`--open` launches the default browser at the loopback URL; omit it to start headlessly. Open `http://127.0.0.1:8000/` (use `http://`, not `https://`; Firefox's HTTPS-Only mode can cause a TLS error against this plaintext port). The live backend log is at `http://127.0.0.1:8000/admin/` (also under **Help → Backend log**). Press **Create**, add components, subscribe, then **Run** (the backend steps the engine every 16 ms while running and the page polls snapshots every 50 ms). **Save file** / **Load file** write and read circuit JSON in `circuits/`; names are restricted to letters, digits, `_` and `-`. Set `ELECTROW_ENGINE` to override the worker path and `ELECTROW_CIRCUITS_DIR` to relocate the circuit folder.

For LAN access during development, bind a non-loopback address; Host/Origin checks are then relaxed automatically, or pass `--allow-lan` explicitly:

```sh
./launch.sh --address 0.0.0.0 --port 8090
# or: python -m backend --host 0.0.0.0 --port 8090 --allow-lan
```

Open `http://<host-ip>:8090/` from the client. The default `python -m backend` invocation stays loopback-only. LAN mode prints a warning and is intended for trusted networks; host/origin hardening can be added later as an optional release feature.

Python talks to the Rust worker over a versioned newline-delimited JSON protocol; the worker owns all simulation state. Measured boundary overhead with `python tests/boundary_bench.py` on the development machine: `snapshot` mean 0.104 ms (p95 0.219 ms), single `step` mean 0.092 ms (p95 0.158 ms) over hundreds of round trips. See [Docs/architecture.md](Docs/architecture.md) and [Docs/interface-schema.json](Docs/interface-schema.json).

## PowerShell scripts

| Script | Purpose |
|---|---|
| `./checkreqs.ps1` | Check cargo, rustc and Python 3.12+ |
| `./winbuild.ps1` | Check requirements, build Rust, syntax-check Python and JS |
| `./test.ps1` | Build, run Rust unit tests and the real-worker HTTP smoke test |
| `./winlaunch.ps1` | Build and launch the local server; `-NoBuild` skips the build, `-Port 8001` changes the port, `-Open` opens the browser |
| `./release.sh` | Full offline verification, source archive, worker binary and clean-environment smoke tests |

## License

Electrow's original code is licensed under the GNU General Public License, version 3 or (at your option) any later version.

SPDX-License-Identifier: `GPL-3.0-or-later`

See [LICENSE](LICENSE) for the full GNU GPL version 3 text. Imported material must retain applicable upstream copyright notices and licensing terms.

See [REFERENCES.md](REFERENCES.md) for candidate upstream repositories, [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party notices, and [PROVENANCE.md](PROVENANCE.md) for code import records.

Electrow is an independent project and is not an official Falstad release.
