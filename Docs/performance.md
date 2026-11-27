# Task 09 — performance baseline and optimization report

Status: measured 2026-11-27. Baseline correctness was gated by Task 08 (102 tests) before any optimization, and the full suite was rerun after each change.

## Environment

- Linux `slowbox` 6.0.0-43-generic x86_64, 8 logical CPUs, AMD Ryzen 5 3400G.
- rustc/cargo 1.93.1, Python 3.14.4, Node 24.20.0.
- All engine numbers are release builds (`cargo run --release --example perf_bench`), 3 repeats per case. Screenshots/logs go to `Temp/`.
- `perf stat` was unavailable (`perf_event_paranoid=4`), so profiling uses opt-in software timers in `Circuit.perf` (stamp, solve, clone, analyze). They are disabled by default and add no work to normal runs.
- Reference comparison was **not possible**: the donor snapshots are Java/GWT source trees with no runnable binary. No reference timing is claimed.

## Reproduce

```sh
cargo test --workspace
cargo run --release --example perf_bench -- --out Temp/perf.json
python tests/boundary_bench.py
./tests/run_browser_ui.sh
```

The harness covers a divider, an RC circuit, 40- and 120-resistor ladders, a 10-diode chain, a BJT switch and repeated topology edits. It reports wall time, simulated-seconds/wall-second, accepted steps/s, Newton iterations, RSS delta and per-step p50/p95.

## Baseline vs optimized (median wall time)

| Case | Elements | Steps | Baseline | Optimized | Change |
|---|---:|---:|---:|---:|---:|
| divider-small | 4 | 10,000 | 21.7 ms | 14.5 ms | −33% |
| rc-small | 4 | 10,000 | 22.0 ms | 14.2 ms | −35% |
| ladder-40 | 43 | 500 | 48.2 ms | 26.4 ms | −45% |
| ladder-120 | 123 | 100 | 151.8 ms | 78.7 ms | −48% |
| diodes-10 | 22 | 500 | 8.9 ms | 6.7 ms | −25% |
| bjt-switch | 5 | 500 | 1.6 ms | 1.1 ms | −31% |
| topology-edits | 60 | 120 edits | 12.6 ms | 10.3 ms | −18% |

Per-step p50 fell from 81 µs to 53 µs (ladder-40) and from 1.45 ms to 0.79 ms (ladder-120). Run-to-run wall variation was under ~3% for the ladders; the small-divider first run was an outlier (~46 ms) due to process warm-up, so medians are reported.

Throughput after optimization: divider 380k steps/s (1.9 simulated s per wall s), RC 705k steps/s (3.5×), ladder-40 19k steps/s, ladder-120 1.3k steps/s, diode chain 73k steps/s, BJT 460k steps/s.

## Profile and changes

Baseline component split (opt-in timers):

- small circuits: stamp 60–64%, clone 19–22%, solve 14–18%;
- ladder-40: solve 77%, stamp 16%, clone 7%;
- ladder-120: solve 95%, stamp 4%, clone 1%;
- topology edits: `analyze()` about 1.4 ms per 20 add/remove cycles.

Two isolated changes, each followed by the full Task 08 suite:

1. **Removed a redundant state clone per step.** `attempt_step` already works on its own `State` clone and only commits on success, so the rollback clone in `Circuit::step` was pure overhead. Failed steps still leave state and time untouched (covered by `determinism_reset_and_failed_steps`).
2. **Reused the matrix buffer and solved in place.** `Matrix` now has `reset()` to zero and resize in place; `Assembly` no longer owns it; `lu_solve_in_place` factors the existing rows/RHS instead of copying them, and a `Circuit.scratch` matrix is reused across Newton iterations and steps. Numerical operations are unchanged.

After optimization the clone share dropped to 10–15% on small circuits and 1–5% on ladders; solve still dominates large networks (72–94%), which is the expected dense-LU cost.

## Bridge, transport and browser

- `snapshot`: mean 0.087 ms, p50 0.078 ms, p95 0.151 ms; ~47 B request, ~227 B response.
- `step` (one step): mean 0.112 ms, p50 0.102 ms, p95 0.173 ms; ~65 B request, ~103 B response.
- Browser snapshot updates during Run: 27 frames in 1 s, p50 50.2 ms, p95 55.6 ms — bounded by the deliberate 50 ms UI poll, not by solver or transport time.
- Solver work is batched into one worker command per tick; Python is never called per stamp or component.

## Bottlenecks and deferred work

- **Sparse/dense crossover:** dense LU is below ~50 ms/s for interactive circuits but scales as O(n³); at 120 nodes it needs ~0.79 ms/step. The measured crossover where sparse methods become worthwhile is around 50–100 unknowns. A sparse or banded solver is the highest-value follow-up, but it must be implemented with the same fixture suite before adoption.
- `analyze()` rebuilds node ids, adjacency and diagnostics on every topology edit; at 60 elements it costs ~70 µs per edit. Acceptable for interactive editing, but incremental updates are possible.
- Small-circuit stamping is now the largest remaining share (70%); flat/contiguous matrix storage is a candidate, unmeasured change.
- No parallelism, GPU or reduced precision was introduced; there is no evidence yet that they are needed for MVP circuit sizes.

## Correctness after optimization

- `cargo test --workspace`: 102 passed, 0 failed, 1 ignored benchmark.
- `python tests/smoke.py`: SMOKE OK.
- `python tests/test_api.py`: 5 passed.
- `./tests/run_browser_ui.sh`: 13/13 browser checks passed.

No numerical fixtures changed tolerance or expected values.
