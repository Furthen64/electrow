import json
import os
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

CIRCUITS_DIR = tempfile.TemporaryDirectory()
os.environ["ELECTROW_CIRCUITS_DIR"] = CIRCUITS_DIR.name

from backend import server
from backend.engine_client import EngineClient, EngineError


def check(condition, message):
    if not condition:
        raise AssertionError(message)


def test_engine_client():
    client = EngineClient()
    try:
        hello = client.hello()
        check(hello["protocol"] == 1, "hello must report protocol 1")

        created = client.command("create", {"circuit_id": "circ-smoke"})
        check(created["revision"] == 1, "create must set revision 1")

        add = client.command(
            "topology_edit",
            {
                "base_revision": 1,
                "ops": [
                    {
                        "op": "add",
                        "element_id": "r1",
                        "kind": "resistor",
                        "node_ids": ["n0", "n1"],
                    }
                ],
            },
        )
        check(add["revision"] == 2, "topology edit must bump revision")

        edited = client.command(
            "parameter_edit",
            {
                "base_revision": 2,
                "element_id": "r1",
                "params": {"resistance_ohm": 1000},
            },
        )
        check(edited["revision"] == 3, "parameter edit must bump revision")

        try:
            client.command(
                "parameter_edit",
                {
                    "base_revision": 2,
                    "element_id": "r1",
                    "params": {"resistance_ohm": 500},
                },
            )
            raise AssertionError("stale revision must be rejected")
        except EngineError as error:
            check(error.code == "stale_revision", "expected stale_revision error")

        client.command(
            "subscribe",
            {
                "stream_id": "scope-1",
                "signals": [
                    {
                        "id": "vout",
                        "element_id": "r1",
                        "quantity": "voltage",
                        "unit": "V",
                    }
                ],
            },
        )
        client.command("step", {"dt_s": 5e-6, "steps": 100})
        snapshot = client.command("snapshot")
        check(
            abs(snapshot["simulation_time_s"] - 5e-4) < 1e-12,
            "step must advance simulated time",
        )
        check(snapshot["revision"] == 3, "snapshot must carry current revision")
        check(snapshot["sequence"] > 0, "snapshot must carry a sequence number")
        check(snapshot["nodes"][0]["id"] == "n0", "snapshot must include topology nodes")
        check(len(snapshot["measurements"]) == 1, "snapshot must include subscribed signals")

        client.command("run")
        check(client.command("snapshot")["running"] is True, "run must set running")
        client.command("pause")
        client.command("reset")
        check(client.command("snapshot")["simulation_time_s"] == 0, "reset must zero time")
    finally:
        client.stop()


def request(url, data=None, headers=None):
    payload = json.dumps(data).encode() if data is not None else None
    request_headers = {"Content-Type": "application/json"}
    if headers:
        request_headers.update(headers)
    req = urllib.request.Request(url, data=payload, headers=request_headers)
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            return response.status, json.loads(response.read())
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read())


def test_http_server():
    httpd = ThreadingHTTPServer(("127.0.0.1", 0), server.Handler)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    base = f"http://127.0.0.1:{httpd.server_address[1]}"
    try:
        with urllib.request.urlopen(base + "/", timeout=5) as response:
            check(b"Electrow" in response.read(), "static index must be served")

        status, health = request(base + "/api/health")
        check(status == 200 and health["protocol"] == 1, "health must reach the engine")

        status, created = request(
            base + "/api/command",
            {"v": 1, "id": "http-1", "cmd": "create", "args": {}},
        )
        check(status == 200 and created["result"]["revision"] == 1, "HTTP create must work")

        status, snapshot = request(base + "/api/snapshot")
        check(status == 200 and snapshot["result"]["circuit_id"] == "circ-1", "HTTP snapshot must work")

        status, stale = request(
            base + "/api/command",
            {
                "v": 1,
                "id": "http-2",
                "cmd": "topology_edit",
                "args": {"base_revision": 99, "ops": []},
            },
        )
        check(
            status == 400 and stale["error"]["code"] == "stale_revision",
            "HTTP must map engine errors to 400",
        )

        status, invalid = request(base + "/api/command", {"v": 2, "id": "x", "cmd": "create"})
        check(status == 400, "HTTP must reject wrong protocol version")

        status, _ = request(
            base + "/api/command",
            {"v": 1, "id": "x", "cmd": "create", "args": {}},
            headers={"Host": "evil.example"},
        )
        check(status == 403, "HTTP must reject foreign Host")
    finally:
        httpd.shutdown()
        httpd.server_close()
        server.engine.stop()


def test_run_driver():
    client = EngineClient()
    try:
        client.command("create", {"circuit_id": "circ-run"})
        client.command("run")
        time.sleep(0.2)
        client.command("pause")
        snapshot = client.command("snapshot")
        check(snapshot["simulation_time_s"] > 0, "run driver must advance simulated time")
        check(snapshot["running"] is False, "pause must stop the driver")
    finally:
        client.stop()


def test_circuit_files():
    httpd = ThreadingHTTPServer(("127.0.0.1", 0), server.Handler)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    base = f"http://127.0.0.1:{httpd.server_address[1]}"
    try:
        document = {
            "format": "electrow-circuit",
            "version": 1,
            "circuit_id": "file-test",
            "elements": [
                {"id": "g", "kind": "ground", "nodes": ["gnd"]},
                {
                    "id": "r",
                    "kind": "resistor",
                    "nodes": ["n1", "gnd"],
                    "params": {"resistance_ohm": 1000},
                },
            ],
        }
        request(
            base + "/api/command",
            {"v": 1, "id": "f0", "cmd": "create", "args": {"circuit_id": "file-test"}},
        )
        status, _ = request(
            base + "/api/command",
            {
                "v": 1,
                "id": "f1",
                "cmd": "load_circuit",
                "args": {"format": "electrow-json", "dump": json.dumps(document)},
            },
        )
        check(status == 200, "load before save must succeed")

        status, saved = request(
            base + "/api/circuit/save", {"v": 1, "id": "f2", "name": "test-circuit"}
        )
        check(status == 200 and saved["result"]["saved"] == "test-circuit.json", "save must work")
        check((Path(CIRCUITS_DIR.name) / "test-circuit.json").is_file(), "file must be written")

        status, listing = request(base + "/api/circuits")
        check(status == 200 and "test-circuit" in listing["names"], "list must include saved name")

        status, _ = request(
            base + "/api/command",
            {"v": 1, "id": "f3", "cmd": "create", "args": {"circuit_id": "overwritten"}},
        )
        status, loaded = request(
            base + "/api/circuit/load", {"v": 1, "id": "f4", "name": "test-circuit"}
        )
        check(status == 200 and loaded["result"]["circuit_id"] == "file-test", "load must work")

        status, bad = request(
            base + "/api/circuit/save", {"v": 1, "id": "f5", "name": "../escape"}
        )
        check(status == 400 and bad["error"]["code"] == "bad_request", "unsafe name must be rejected")

        status, missing = request(
            base + "/api/circuit/load", {"v": 1, "id": "f6", "name": "does-not-exist"}
        )
        check(status == 404, "missing circuit must be 404")
    finally:
        httpd.shutdown()
        httpd.server_close()
        server.engine.stop()


def test_solver_round_trip():
    client = EngineClient()
    try:
        client.command("create", {"circuit_id": "circ-divider"})
        ops = [
            {"op": "add", "element_id": "gnd", "kind": "ground", "node_ids": ["gnd"]},
            {"op": "add", "element_id": "v1", "kind": "voltage_source", "node_ids": ["n1", "gnd"]},
            {"op": "add", "element_id": "r1", "kind": "resistor", "node_ids": ["n1", "n2"]},
            {"op": "add", "element_id": "r2", "kind": "resistor", "node_ids": ["n2", "gnd"]},
        ]
        ack = client.command("topology_edit", {"base_revision": 1, "ops": ops})
        check(ack["revision"] == 2, "divider topology must bump revision")
        for revision, element_id, params in [
            (2, "v1", {"voltage_v": 10}),
            (3, "r1", {"resistance_ohm": 1000}),
            (4, "r2", {"resistance_ohm": 1000}),
        ]:
            edited = client.command(
                "parameter_edit",
                {"base_revision": revision, "element_id": element_id, "params": params},
            )
            check(edited["revision"] == revision + 1, "parameter edit must bump revision")
        client.command(
            "subscribe",
            {
                "stream_id": "scope-1",
                "signals": [
                    {
                        "id": "vout",
                        "element_id": "r2",
                        "quantity": "voltage",
                        "unit": "V",
                    }
                ],
            },
        )
        client.command("step", {"dt_s": 5e-6, "steps": 1})
        snapshot = client.command("snapshot")
        nodes = {node["id"]: node["voltage_v"] for node in snapshot["nodes"]}
        check(abs(snapshot["simulation_time_s"] - 5e-6) < 1e-12, "solver must advance time")
        check(abs(nodes["n2"] - 5.0) < 1e-6, "divider midpoint must be 5 V")
        check(
            abs(snapshot["measurements"][0]["value"] - 5.0) < 1e-6,
            "subscription must read solved voltage",
        )
        check(snapshot["diagnostics"] == [], "valid divider must have no diagnostics")

        client.command("create", {"circuit_id": "circ-float"})
        client.command(
            "topology_edit",
            {
                "base_revision": 1,
                "ops": [
                    {"op": "add", "element_id": "g", "kind": "ground", "node_ids": ["gnd"]},
                    {"op": "add", "element_id": "r", "kind": "resistor", "node_ids": ["a", "b"]},
                ],
            },
        )
        snapshot = client.command("snapshot")
        kinds = [diagnostic["kind"] for diagnostic in snapshot["diagnostics"]]
        check("floating_node" in kinds, "disconnected resistor must report floating nodes")
    finally:
        client.stop()


if __name__ == "__main__":
    test_engine_client()
    test_run_driver()
    test_solver_round_trip()
    test_http_server()
    test_circuit_files()
    print("SMOKE OK: Rust worker, versioned protocol, solver and HTTP round trip")
