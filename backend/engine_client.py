import itertools
import json
import os
import subprocess
import threading
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ENGINE = REPO_ROOT / "target" / "debug" / (
    "electrow-engine.exe" if os.name == "nt" else "electrow-engine"
)
PROTOCOL_VERSION = 1
MAX_REQUEST_BYTES = 1024 * 1024
MAX_RESPONSE_BYTES = 4 * 1024 * 1024


class EngineError(RuntimeError):
    def __init__(self, error):
        self.code = error.get("code", "internal_error")
        self.retryable = error.get("retryable", False)
        super().__init__(error.get("message", self.code))


class EngineClient:
    def __init__(self, executable=None):
        configured = executable or os.environ.get("ELECTROW_ENGINE")
        self.executable = Path(configured) if configured else DEFAULT_ENGINE
        self.process = None
        self.lock = threading.RLock()
        self.ids = itertools.count(1)

    def start(self):
        if self.process is not None and self.process.poll() is None:
            return
        if os.name == "nt" and not self.executable.suffix and not self.executable.is_file():
            self.executable = self.executable.with_suffix(".exe")
        if not self.executable.is_file():
            raise FileNotFoundError(
                f"engine executable not found: {self.executable} (run cargo build)"
            )
        self.process = subprocess.Popen(
            [str(self.executable)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

    def hello(self):
        return self.command("hello")

    def command(self, cmd, args=None, request_id=None):
        with self.lock:
            return self._command_locked(cmd, args, request_id)

    def _command_locked(self, cmd, args=None, request_id=None):
        self.start()
        if self.process.poll() is not None:
            raise RuntimeError("engine process has exited")
        request = {
            "v": PROTOCOL_VERSION,
            "id": request_id or f"py-{next(self.ids)}",
            "cmd": cmd,
            "args": args or {},
        }
        line = json.dumps(request, separators=(",", ":"))
        if len(line.encode("utf-8")) > MAX_REQUEST_BYTES:
            raise ValueError("request exceeds maximum size")
        self.process.stdin.write(line + "\n")
        self.process.stdin.flush()
        response_line = self.process.stdout.readline(MAX_RESPONSE_BYTES + 2)
        if not response_line:
            raise RuntimeError("engine closed the connection")
        if not response_line.endswith("\n"):
            raise RuntimeError("engine response exceeds maximum size")
        response = json.loads(response_line)
        if response.get("v") != PROTOCOL_VERSION:
            raise RuntimeError("unsupported engine protocol version")
        if response.get("id") != request["id"]:
            raise RuntimeError("engine response id mismatch")
        if not response.get("ok"):
            raise EngineError(response.get("error", {}))
        return response.get("result", {})

    def stop(self):
        with self.lock:
            if self.process is None:
                return
            if self.process.poll() is None:
                try:
                    self._command_locked("shutdown")
                except (OSError, RuntimeError, ValueError):
                    self.process.terminate()
                try:
                    self.process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    self.process.kill()
            self.process = None
