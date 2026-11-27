"""Measure Python<->Rust worker command overhead with the real binary."""

import json
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
    request_bytes = []
    response_bytes = []
    for _ in range(count):
        started = time.perf_counter()
        result = client.command(cmd, args)
        samples.append((time.perf_counter() - started) * 1000.0)
        request_bytes.append(
            len(json.dumps({"v": 1, "id": "bench", "cmd": cmd, "args": args},
                           separators=(",", ":")).encode("utf-8"))
        )
        response_bytes.append(
            len(json.dumps({"v": 1, "id": "bench", "ok": True, "result": result},
                           separators=(",", ":")).encode("utf-8"))
        )
    samples.sort()
    return {
        "mean_ms": statistics.mean(samples),
        "p50_ms": samples[len(samples) // 2],
        "p95_ms": samples[int(len(samples) * 0.95)],
        "request_bytes": statistics.mean(request_bytes),
        "response_bytes": statistics.mean(response_bytes),
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
                f"p50={result['p50_ms']:.3f} ms p95={result['p95_ms']:.3f} ms "
                f"request={result['request_bytes']:.0f}B response={result['response_bytes']:.0f}B"
            )
    finally:
        client.stop()


if __name__ == "__main__":
    main()
