import argparse
import json
import mimetypes
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse

from .engine_client import PROTOCOL_VERSION, EngineClient, EngineError, SessionLostError

WEB_ROOT = Path(__file__).resolve().parents[1] / "web"
MAX_BODY_BYTES = 1024 * 1024
ALLOWED_HOSTS = {"127.0.0.1", "localhost", "::1"}
CLIENT_ERROR_CODES = {
    "unsupported_version",
    "bad_request",
    "unknown_command",
    "stale_revision",
    "unknown_element",
    "unknown_stream",
    "bad_document",
    "invalid_document",
    "no_document",
}
engine = EngineClient()


class Handler(BaseHTTPRequestHandler):
    def _send(self, status, body, content_type):
        payload = body.encode("utf-8") if isinstance(body, str) else body
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def _send_json(self, status, payload):
        self._send(status, json.dumps(payload), "application/json")

    def _error(self, status, code, message, request_id="invalid", retryable=False):
        self._send_json(
            status,
            {
                "v": PROTOCOL_VERSION,
                "id": request_id,
                "ok": False,
                "error": {
                    "code": code,
                    "message": message,
                    "retryable": retryable,
                },
            },
        )

    def _host_allowed(self):
        host = self.headers.get("Host", "").strip()
        name = host[1:-1] if host.startswith("[") else host.split(":")[0]
        if name not in ALLOWED_HOSTS:
            return False
        origin = self.headers.get("Origin")
        if not origin:
            return True
        parsed = urlparse(origin)
        return parsed.scheme == "http" and parsed.hostname in ALLOWED_HOSTS

    def _reject_host(self):
        self._error(403, "host_not_allowed", "host not allowed")
        return False

    def do_GET(self):
        if not self._host_allowed():
            self._reject_host()
            return
        path = urlparse(self.path).path
        if path == "/api/health":
            try:
                result = engine.hello()
            except (OSError, RuntimeError, ValueError, SessionLostError) as exc:
                self._error(503, "engine_unavailable", str(exc))
                return
            self._send_json(
                200,
                {
                    "v": PROTOCOL_VERSION,
                    "status": "ok",
                    "engine": str(engine.executable),
                    "engine_version": result.get("engine_version"),
                    "protocol": result.get("protocol"),
                },
            )
            return
        if path == "/api/snapshot":
            try:
                result = engine.command("snapshot")
            except (OSError, RuntimeError, ValueError, SessionLostError) as exc:
                self._error(503, "engine_unavailable", str(exc))
                return
            self._send_json(
                200, {"v": PROTOCOL_VERSION, "id": "snapshot", "ok": True, "result": result}
            )
            return
        self._static(path)

    def do_POST(self):
        if not self._host_allowed():
            self._reject_host()
            return
        if urlparse(self.path).path != "/api/command":
            self._error(404, "not_found", "not found")
            return
        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            self._error(400, "bad_request", "invalid content length")
            return
        if length <= 0 or length > MAX_BODY_BYTES:
            self._error(413, "invalid_request_size", "invalid request size")
            return
        body = self.rfile.read(length)
        if len(body) != length:
            self._error(400, "bad_request", "truncated request")
            return
        try:
            request = json.loads(body)
            if not isinstance(request, dict):
                raise ValueError("request must be an object")
            if request.get("v") != PROTOCOL_VERSION:
                raise ValueError("unsupported protocol version")
            request_id = request.get("id")
            cmd = request.get("cmd")
            args = request.get("args", {})
            if not isinstance(request_id, str) or not request_id or len(request_id) > 128:
                raise ValueError("id must be a non-empty string of at most 128 characters")
            if not isinstance(cmd, str) or not cmd:
                raise ValueError("cmd must be a non-empty string")
            if not isinstance(args, dict):
                raise ValueError("args must be an object")
        except (ValueError, json.JSONDecodeError) as exc:
            self._error(400, "bad_request", str(exc))
            return

        try:
            result = engine.command(cmd, args, request_id=request_id)
        except SessionLostError as exc:
            self._error(503, "session_lost", str(exc), request_id, retryable=True)
            return
        except EngineError as exc:
            status = 400 if exc.code in CLIENT_ERROR_CODES else 503
            self._error(status, exc.code, str(exc), request_id, retryable=exc.retryable)
            return
        except (OSError, RuntimeError, ValueError) as exc:
            self._error(503, "engine_unavailable", str(exc), request_id)
            return
        self._send_json(
            200, {"v": PROTOCOL_VERSION, "id": request_id, "ok": True, "result": result}
        )

    def _static(self, path):
        if path in ("", "/"):
            path = "/index.html"
        candidate = (WEB_ROOT / path.lstrip("/")).resolve()
        if not candidate.is_relative_to(WEB_ROOT.resolve()) or not candidate.is_file():
            self._error(404, "not_found", "not found")
            return
        content_type = mimetypes.guess_type(candidate.name)[0] or "application/octet-stream"
        self._send(200, candidate.read_bytes(), content_type)


def main():
    parser = argparse.ArgumentParser(description="Electrow local backend")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8000)
    args = parser.parse_args()

    server = ThreadingHTTPServer((args.host, args.port), Handler)
    server.daemon_threads = True
    print(f"Electrow UI at http://{args.host}:{args.port}/")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
        engine.stop()


if __name__ == "__main__":
    main()
