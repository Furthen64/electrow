"""Run one component test scenario against the real Rust engine.

Usage: python tests/check_component.py <scenario.json>

Scenario schema (see tests/ComponentTests/*.json):
  name:        short label, e.g. "switch"
  ops:         list of add ops applied in one topology_edit
  params:      list of {"element_id": ..., "params": {key: value}} applied in order
  step:        {"dt_s": ..., "steps": ...} or null
  expect:
    diagnostics: "empty" (assert no diagnostics) or "any" (assert diagnostics present)
    nodes: {node_id: {"expected": value, "tolerance": value (optional, default 0.05)}}
    measurements: [{"element_id": ..., "quantity": "voltage"|"current",
                     "unit": "V"|"A", "expected": value, "tolerance": value (default 0.05)}]
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from backend.engine_client import EngineClient, EngineError


def run(path: Path) -> list[str]:
    failures: list[str] = []
    scenario = json.loads(path.read_text(encoding="utf-8"))

    try:
        client = EngineClient()
        try:
            client.command("create", {"circuit_id": f"component-{scenario['name']}"})
            revision = 1
            ops = scenario.get("ops", [])
            if ops:
                client.command("topology_edit", {"base_revision": revision, "ops": ops})
                revision += 1
            for edit in scenario.get("params", []):
                client.command(
                    "parameter_edit",
                    {"base_revision": revision, "element_id": edit["element_id"], "params": edit["params"]},
                )
                revision += 1

            step = scenario.get("step")
            if step:
                client.command("step", {"dt_s": step["dt_s"], "steps": step.get("steps", 1)})

            expect = scenario.get("expect", {})
            measurements = expect.get("measurements", [])
            if measurements:
                signals = []
                for index, entry in enumerate(measurements):
                    quantity = entry["quantity"]
                    signals.append(
                        {
                            "id": f"m-{index}",
                            "element_id": entry["element_id"],
                            "quantity": quantity,
                            "unit": entry.get("unit", "V" if quantity == "voltage" else "A"),
                        }
                    )
                client.command("subscribe", {"stream_id": "component-test", "signals": signals})

            snapshot = client.command("snapshot")

            diagnostics = snapshot.get("diagnostics", [])
            expected_diagnostics = expect.get("diagnostics")
            if expected_diagnostics == "empty" and diagnostics:
                failures.append(f"diagnostics should be empty, got {diagnostics}")
            if expected_diagnostics == "any" and not diagnostics:
                failures.append("expected diagnostics but got none")

            nodes = {node["id"]: node["voltage_v"] for node in snapshot.get("nodes", [])}
            for node_id, rule in expect.get("nodes", {}).items():
                value = nodes.get(node_id)
                if value is None:
                    failures.append(f"node {node_id!r} missing from snapshot")
                    continue
                tolerance = rule.get("tolerance", 0.05)
                detail = f"node {node_id}: expected {rule['expected']} +/- {tolerance}, got {value}"
                if abs(value - rule["expected"]) > tolerance:
                    failures.append(detail)

            snapshot_measurements = {
                (measurement["element_id"], measurement["quantity"]): measurement["value"]
                for measurement in snapshot.get("measurements", [])
            }
            for entry in measurements:
                key = (entry["element_id"], entry["quantity"])
                value = snapshot_measurements.get(key)
                if value is None:
                    failures.append(f"measurement {key} missing from snapshot")
                    continue
                tolerance = entry.get("tolerance", 0.05)
                detail = (
                    f"measurement {entry['element_id']}/{entry['quantity']}: "
                    f"expected {entry['expected']} +/- {tolerance}, got {value}"
                )
                if abs(value - entry["expected"]) > tolerance:
                    failures.append(detail)
        finally:
            client.stop()
    except (EngineError, RuntimeError, FileNotFoundError, ValueError) as error:
        failures.append(f"scenario error: {error}")

    return failures


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: python tests/check_component.py <scenario.json>")
        return 2
    scenario_path = Path(sys.argv[1])
    failures = run(scenario_path)
    name = json.loads(scenario_path.read_text(encoding="utf-8")).get("name", scenario_path.stem)
    if failures:
        for failure in failures:
            print(f"FAIL {name}: {failure}")
        return 1
    print(f"PASS {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
