# Task 11 — Local application packaging and release readiness

Status: pending. Depends on 06–10.

Document supported Windows/Linux targets, Rust and Python versions and reproducible build steps. Provide simple local launch flow serving the UI on loopback and opening a browser if desired. Choose packaging after measuring build/runtime dependencies; avoid shipping NW.js/Electron unless later justified. Pin dependencies, test on clean environments and verify shutdown, port conflicts and offline operation.

Before distributing source or binaries, apply the import gate to all selected code, dependencies, examples, fonts/icons/assets and build scripts. Preserve original notices, record modifications, include required license texts and satisfy GPL corresponding-source obligations, including relevant build/install scripts and applicable third-party obligations. GPL-3.0-or-later remains Electrow's original-code policy; review any GPLv3-only material before representing the combined distribution as or-later. Do not package reference folders by default.

Acceptance: documented build and launch, clean-environment smoke tests, passing numerical and UI regressions, dependency/license inventory, release artifacts and corresponding-source delivery plan. Record known limitations and unsupported components in release notes.
