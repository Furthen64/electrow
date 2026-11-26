import itertools
import json
import os
import select
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
READ_TIMEOUT_S = 5.0
DRIVER_TICK_S = 0.016
DRIVER_STEPS_PER_TICK = 4
DRIVER_BASE_DT_S = 5e-6


class SessionLostError(RuntimeError):
    def __init__(self):
        super().__init__("engine session was restarted; circuit state was lost")


class EngineError(RuntimeError):
    def __init__(self, error):
        if not isinstance(error, dict):
            error = {}
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
        self.running = False
        self._driver_stop = threading.Event()
        self._driver_thread = None

    def _spawn(self):
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
            bufsize=-1,
        )

    def _ensure_process(self):
        if self.process is not None and self.process.poll() is None:
            return
        lost = self.process is not None
        self._spawn()
        if lost:
            self.running = False
            raise SessionLostError()

    def _start_driver(self):
        if self._driver_thread is not None:
            return
        self._driver_stop.clear()
        self._driver_thread = threading.Thread(target=self._driver_loop, daemon=True)
        self._driver_thread.start()

    def _driver_loop(self):
        while not self._driver_stop.wait(DRIVER_TICK_S):
            if not self.running:
                continue
            try:
                self.command("step", {"dt_s": DRIVER_BASE_DT_S, "steps": DRIVER_STEPS_PER_TICK})
            except (SessionLostError, EngineError, OSError, RuntimeError, ValueError):
                self.running = False

    def start(self):
        with self.lock:
            self._ensure_process()
            self._start_driver()

    def hello(self):
        return self.command("hello")

    def command(self, cmd, args=None, request_id=None):
        with self.lock:
            result = self._command_locked(cmd, args, request_id)
            if cmd == "run":
                self.running = True
            elif cmd in ("pause", "reset", "create", "load_circuit"):
                self.running = False
            return result

    def _command_locked(self, cmd, args=None, request_id=None):
        self._ensure_process()
        self._start_driver()
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
        self.process.stdin.write((line + "\n").encode("utf-8"))
        self.process.stdin.flush()
        ready, _, _ = select.select([self.process.stdout], [], [], READ_TIMEOUT_S)
        if not ready:
            self._reset_worker()
            raise RuntimeError(f"engine did not respond within {READ_TIMEOUT_S}s")
        raw = self.process.stdout.readline(MAX_RESPONSE_BYTES + 2)
        if not raw:
            self._reset_worker()
            raise RuntimeError("engine closed the connection")
        if not raw.endswith(b"\n"):
            self._drain_line()
            self._reset_worker()
            raise RuntimeError("engine response exceeds maximum size")
        try:
            response = json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, ValueError) as exc:
            raise RuntimeError("invalid engine response") from exc
        if response.get("v") != PROTOCOL_VERSION:
            raise RuntimeError("unsupported engine protocol version")
        if response.get("id") != request["id"]:
            raise RuntimeError("engine response id mismatch")
        if not response.get("ok"):
            raise EngineError(response.get("error", {}))
        return response.get("result", {})

    def _drain_line(self):
        while True:
            chunk = self.process.stdout.readline(MAX_RESPONSE_BYTES + 2)
            if not chunk or chunk.endswith(b"\n"):
                return

    def _reset_worker(self):
        process = self.process
        self.process = None
        self.running = False
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=2)

    def stop(self):
        with self.lock:
            self._driver_stop.set()
            process = self.process
            if process is not None and process.poll() is None:
                try:
                    self._command_locked("shutdown")
                except (OSError, RuntimeError, ValueError, SessionLostError):
                    process.terminate()
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=2)
            self.process = None
            self.running = False
        if self._driver_thread is not None:
            self._driver_thread.join(timeout=2)
            self._driver_thread = None
