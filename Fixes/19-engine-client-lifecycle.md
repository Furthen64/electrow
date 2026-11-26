# Fix 19 — Harden the engine client lifecycle

Status: pending
Severity: medium-high (server wedge / protocol desync / silent state loss)

## Problem
Three related issues in `engine_client.py`:
- `readline()` has no timeout and runs under `RLock`, so a hung engine call blocks every HTTP handler; non-daemon `ThreadingHTTPServer` threads can prevent clean shutdown.
- A response larger than `MAX_RESPONSE_BYTES` returns a partial line; the unread remainder corrupts all subsequent responses with no resync path.
- `start()` transparently respawns a fresh engine after a crash, losing circuit/revision/subscription state; the browser keeps a stale `lastRevision` and never recovers.

## Evidence
- `backend/engine_client.py:29,53-54,71-80` — lock, blocking read, partial read.
- `backend/engine_client.py:32-47` — silent restart.
- `backend/server.py:167` — ThreadingHTTPServer with default non-daemon threads.
- `web/app.js:10,39` — cached revision with no recovery.

## Fix
- Add a timeout to the worker read; on timeout treat the session as failed (kill/restart) and return a 503 with a retryable error instead of holding the lock indefinitely.
- On an oversized/partial response, drain to newline or restart the worker, then return an explicit protocol error; never parse a partial line.
- On restart, expose a session-lost signal (e.g. a response code the UI handles by clearing `lastRevision` and re-creating the circuit), rather than silently continuing.
- Make handler threads daemon or add bounded shutdown handling.

## Acceptance
- A hung worker produces a 503 within the timeout instead of blocking other requests.
- An oversized response cannot poison the next command.
- After a worker restart the browser can recover automatically.
- `python tests/smoke.py` passes.
