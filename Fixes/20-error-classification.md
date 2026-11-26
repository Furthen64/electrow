# Fix 20 — Correct backend error classification

Status: complete (2026-11-26). Added missing client error codes to `server.py`; every error response now uses the standard object envelope (`id`, `code`, `message`, `retryable`); `/api/snapshot` includes `id`. Smoke test confirms 400/403 mapping.
Severity: low-medium (wrong HTTP status codes)

## Problem
`CLIENT_ERROR_CODES` omits engine codes `bad_document`, `invalid_document` and `no_document`, so malformed user input is returned as HTTP 503 "engine unavailable" instead of 400. Conversely the schema declares `solver_failed`, which the engine never emits. Some server error payloads are bare strings while others are objects, so `web/app.js` renders "undefined: undefined".

## Evidence
- `backend/server.py:13-20,124` — status mapping.
- `engine/src/lib.rs:270,275,315,361` — emitted codes.
- `backend/server.py:48,88,93,96,100,118` — string error payloads.
- `web/app.js:33-34` — assumes `data.error` is an object.

## Fix
Add all client-caused engine codes to `CLIENT_ERROR_CODES`; reserve 503 for engine unavailability. Standardize every error response body as `{"v":..., "id":..., "ok":false, "error":{"code":..., "message":...}}`. Update `web/app.js` to handle both string and object error shapes defensively. Align with Fix 21 on the schema.

## Acceptance
- Malformed document loads return 400; true engine failures return 503.
- UI shows a readable message for host/content-length/protocol errors.
- `python tests/smoke.py` passes.
