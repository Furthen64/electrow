"""Python-side API tests against the real Rust worker and HTTP server."""

import json
import os
import sys
import tempfile
import threading
import unittest
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


def http_json(base, path, payload=None, headers=None):
    request_headers = {"Content-Type": "application/json"}
    if headers:
        request_headers.update(headers)
    data = json.dumps(payload).encode() if payload is not None else None
    request = urllib.request.Request(base + path, data=data, headers=request_headers)
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            return response.status, json.loads(response.read())
    except urllib.error.HTTPError as error:
        with error:
            return error.code, json.loads(error.read())


class EngineClientTests(unittest.TestCase):
    def setUp(self):
        self.client = EngineClient()

    def tearDown(self):
        self.client.stop()

    def test_protocol_and_revisions(self):
        hello = self.client.hello()
        self.assertEqual(hello["protocol"], 1)
        created = self.client.command("create", {"circuit_id": "api-test"})
        self.assertEqual(created["revision"], 1)
        with self.assertRaises(EngineError) as stale:
            self.client.command("parameter_edit", {
                "base_revision": 99,
                "element_id": "r1",
                "params": {"resistance_ohm": 1000},
            })
        self.assertEqual(stale.exception.code, "stale_revision")

    def test_unknown_command_and_invalid_document(self):
        self.client.command("create")
        with self.assertRaises(EngineError) as unknown:
            self.client.command("nonexistent")
        self.assertEqual(unknown.exception.code, "unknown_command")
        with self.assertRaises(EngineError) as invalid:
            self.client.command("load_circuit", {
                "format": "electrow-json",
                "dump": json.dumps({"format": "electrow-circuit", "version": 99}),
            })
        self.assertEqual(invalid.exception.code, "invalid_document")

    def test_step_snapshot_and_subscription(self):
        self.client.command("create", {"circuit_id": "api-step"})
        self.client.command("topology_edit", {
            "base_revision": 1,
            "ops": [{
                "op": "add",
                "element_id": "r1",
                "kind": "resistor",
                "node_ids": ["n0", "n1"],
            }],
        })
        self.client.command("subscribe", {
            "stream_id": "scope-1",
            "signals": [{
                "id": "vout",
                "element_id": "r1",
                "quantity": "voltage",
                "unit": "V",
            }],
        })
        self.client.command("step", {"dt_s": 5e-6, "steps": 10})
        snapshot = self.client.command("snapshot")
        self.assertAlmostEqual(snapshot["simulation_time_s"], 5e-5, places=12)
        self.assertEqual(len(snapshot["measurements"]), 1)
        self.client.command("reset")
        self.assertEqual(self.client.command("snapshot")["simulation_time_s"], 0.0)


class HttpApiTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.httpd = ThreadingHTTPServer(("127.0.0.1", 0), server.Handler)
        cls.thread = threading.Thread(target=cls.httpd.serve_forever, daemon=True)
        cls.thread.start()
        cls.base = f"http://127.0.0.1:{cls.httpd.server_address[1]}"

    @classmethod
    def tearDownClass(cls):
        cls.httpd.shutdown()
        cls.httpd.server_close()
        server.engine.stop()

    def test_health_snapshot_and_command(self):
        status, health = http_json(self.base, "/api/health")
        self.assertEqual(status, 200)
        self.assertEqual(health["protocol"], 1)
        status, created = http_json(self.base, "/api/command", {
            "v": 1, "id": "py-1", "cmd": "create", "args": {},
        })
        self.assertEqual(status, 200)
        revision = created["result"]["revision"]
        self.assertGreaterEqual(revision, 1)
        status, snapshot = http_json(self.base, "/api/snapshot")
        self.assertEqual(status, 200)
        self.assertEqual(snapshot["result"]["revision"], revision)

    def test_client_errors_and_host_check(self):
        http_json(self.base, "/api/command", {
            "v": 1, "id": "setup", "cmd": "create", "args": {},
        })
        status, response = http_json(self.base, "/api/command", {
            "v": 1, "id": "py-2", "cmd": "topology_edit",
            "args": {"base_revision": 42, "ops": []},
        })
        self.assertEqual(status, 400)
        self.assertEqual(response["error"]["code"], "stale_revision")
        status, _ = http_json(self.base, "/api/health", headers={"Host": "evil.example"})
        self.assertEqual(status, 403)
        server.allow_lan = True
        try:
            status, health = http_json(
                self.base,
                "/api/health",
                headers={"Host": "192.168.1.155:8090", "Origin": "http://192.168.1.155:8090"},
            )
            self.assertEqual(status, 200)
            self.assertEqual(health["protocol"], 1)
        finally:
            server.allow_lan = False
        status, bad_document = http_json(self.base, "/api/command", {
            "v": 1,
            "id": "py-3",
            "cmd": "load_circuit",
            "args": {"format": "electrow-json", "dump": '{"version": 99}'},
        })
        self.assertEqual(status, 400)
        self.assertEqual(bad_document["error"]["code"], "invalid_document")


if __name__ == "__main__":
    unittest.main(verbosity=2)
