const PROTOCOL_VERSION = 1;

export class Transport {
  constructor() {
    this.revision = 0;
    this.sequence = -1;
    this.nextId = 1;
    this.listeners = new Set();
    this.pollTimer = null;
    this.onSessionLost = null;
    this.onStateChange = null;
  }

  onSnapshot(listener) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  emit(snapshot) {
    for (const listener of this.listeners) listener(snapshot);
  }

  setPolling(enabled) {
    if (enabled && this.pollTimer === null) {
      this.pollTimer = setInterval(() => this.send("snapshot", {}, { silent: true }), 50);
    } else if (!enabled && this.pollTimer !== null) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }
    if (this.onStateChange) this.onStateChange({ polling: this.pollTimer !== null });
  }

  async send(cmd, args = {}, { silent = false } = {}) {
    const request = {
      v: PROTOCOL_VERSION,
      id: `web-${this.nextId++}`,
      cmd,
      args,
    };
    let result;
    try {
      const response = await fetch("/api/command", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      });
      const data = await response.json();
      if (!response.ok || data.ok === false) {
        const error = data.error || { code: "http", message: `HTTP ${response.status}` };
        if (error.code === "session_lost") {
          this.setPolling(false);
          this.revision = 0;
          this.sequence = -1;
          if (this.onSessionLost) this.onSessionLost();
        }
        return { ok: false, error };
      }
      result = data.result;
    } catch (error) {
      return { ok: false, error: { code: "transport", message: String(error) } };
    }
    if (typeof result.revision === "number") this.revision = result.revision;
    if (result.type === "snapshot") {
      if (result.sequence <= this.sequence) return { ok: true, stale: true, result };
      this.sequence = result.sequence;
      this.emit(result);
    } else {
      this.send("snapshot", {}, { silent: true });
    }
    return { ok: true, result };
  }

  async health() {
    try {
      const response = await fetch("/api/health");
      const data = await response.json();
      return { ok: response.ok, data };
    } catch (error) {
      return { ok: false, error: String(error) };
    }
  }

  async fileCommand(path, name) {
    try {
      const response = await fetch(path, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ v: PROTOCOL_VERSION, id: `web-${this.nextId++}`, name }),
      });
      const data = await response.json();
      if (!response.ok || data.ok === false) {
        return { ok: false, error: data.error || { code: "http", message: `HTTP ${response.status}` } };
      }
      return { ok: true, result: data.result };
    } catch (error) {
      return { ok: false, error: { code: "transport", message: String(error) } };
    }
  }

  async listFiles() {
    try {
      const response = await fetch("/api/circuits");
      const data = await response.json();
      return data.names || [];
    } catch (error) {
      return [];
    }
  }
}
