import json
import sys
import threading
import urllib.error
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

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


if __name__ == "__main__":
    test_engine_client()
    test_http_server()
    print("SMOKE OK: Rust worker, versioned protocol and local HTTP round trip")
