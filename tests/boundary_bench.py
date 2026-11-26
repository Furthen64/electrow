"""Measure Python<->Rust worker command overhead with the real binary."""

import statistics
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from backend.engine_client import EngineClient


def measure(client, cmd, args, count, warmup=20):
    for _ in range(warmup):
        client.command(cmd, args)
    samples = []
    for _ in range(count):
        started = time.perf_counter()
        client.command(cmd, args)
        samples.append((time.perf_counter() - started) * 1000.0)
    samples.sort()
    return {
        "mean_ms": statistics.mean(samples),
        "p50_ms": samples[len(samples) // 2],
        "p95_ms": samples[int(len(samples) * 0.95)],
    }


def main():
    client = EngineClient()
    try:
        client.command("create", {"circuit_id": "bench"})
        for cmd, args, count in [
            ("snapshot", {}, 500),
            ("step", {"dt_s": 5e-6, "steps": 1}, 200),
        ]:
            result = measure(client, cmd, args, count)
            print(
                f"{cmd}: n={count} mean={result['mean_ms']:.3f} ms "
                f"p50={result['p50_ms']:.3f} ms p95={result['p95_ms']:.3f} ms"
            )
    finally:
        client.stop()


if __name__ == "__main__":
    main()
