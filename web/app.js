const els = {
  status: document.getElementById("status"),
  response: document.getElementById("response"),
  snapshot: document.getElementById("snapshot"),
  dump: document.getElementById("dump"),
  resistance: document.getElementById("resistance"),
  filename: document.getElementById("filename"),
  saved: document.getElementById("saved-circuits"),
};

let nextId = 1;
let lastRevision = 0;
let lastSequence = -1;
let pollTimer = null;

function startPolling() {
  if (pollTimer !== null) return;
  pollTimer = setInterval(() => send("snapshot"), 50);
}

function stopPolling() {
  if (pollTimer !== null) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

function renderSnapshot(snapshot) {
  els.snapshot.textContent =
    `circuit: ${snapshot.circuit_id}\n` +
    `revision: ${snapshot.revision}  sequence: ${snapshot.sequence}\n` +
    `simulated time: ${snapshot.simulation_time_s} s  step: ${snapshot.time_step_s} s\n` +
    `running: ${snapshot.running}  status: ${snapshot.status}\n` +
    `nodes: ${JSON.stringify(snapshot.nodes)}\n` +
    `measurements: ${JSON.stringify(snapshot.measurements)}`;
}

async function send(cmd, args = {}) {
  const request = { v: 1, id: `web-${nextId++}`, cmd, args };
  try {
    const result = await fetch("/api/command", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
    const data = await result.json();
    if (!result.ok || data.ok === false) {
      const error = data.error || { code: "http", message: `HTTP ${result.status}` };
      els.response.textContent = `${error.code}: ${error.message}`;
      els.status.textContent = "error";
      if (error.code === "session_lost") {
        stopPolling();
        lastRevision = 0;
        lastSequence = -1;
        send("create");
      }
      return null;
    }
    const response = data.result;
    if (typeof response.revision === "number") lastRevision = response.revision;
    els.response.textContent = JSON.stringify(response, null, 2);
    els.status.textContent = "connected";
    if (response && response.type === "snapshot") {
      if (response.sequence <= lastSequence) {
        els.status.textContent = "stale frame dropped";
        return response;
      }
      lastSequence = response.sequence;
      renderSnapshot(response);
    }
    return response;
  } catch (error) {
    els.response.textContent = String(error);
    els.status.textContent = "disconnected";
    return null;
  }
}

async function fileCommand(path, name) {
  const payload = { v: 1, id: `web-${nextId++}`, name };
  try {
    const result = await fetch(path, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    const data = await result.json();
    if (!result.ok || data.ok === false) {
      const error = data.error || { code: "http", message: `HTTP ${result.status}` };
      els.response.textContent = `${error.code}: ${error.message}`;
      els.status.textContent = "error";
      return null;
    }
    els.response.textContent = JSON.stringify(data.result, null, 2);
    els.status.textContent = "connected";
    return data.result;
  } catch (error) {
    els.response.textContent = String(error);
    els.status.textContent = "disconnected";
    return null;
  }
}

async function refreshFiles() {
  try {
    const data = await (await fetch("/api/circuits")).json();
    els.saved.replaceChildren(new Option("saved circuits", ""));
    for (const name of data.names || []) els.saved.add(new Option(name, name));
  } catch (error) {
    els.response.textContent = String(error);
  }
}

function bind(id, cmd, args) {
  const element = document.getElementById(id);
  element.addEventListener("click", () =>
    send(cmd, typeof args === "function" ? args() : args || {})
  );
}

bind("hello", "hello");
bind("create", "create");
bind("add", "topology_edit", () => ({
  base_revision: lastRevision,
  ops: [{ op: "add", element_id: "r1", kind: "resistor", node_ids: ["n0", "n1"] }],
}));
bind("params", "parameter_edit", () => ({
  base_revision: lastRevision,
  element_id: "r1",
  params: { resistance_ohm: Number(els.resistance.value) },
}));
bind("load", "load_circuit", () => ({
  dump: els.dump.value,
  format: els.dump.value.trimStart().startsWith("<") ? "xml" : "text",
  base_revision: lastRevision,
}));
bind("subscribe", "subscribe", () => ({
  stream_id: "scope-1",
  signals: [{ id: "vout", element_id: "r1", quantity: "voltage", unit: "V" }],
}));
bind("unsubscribe", "unsubscribe", { stream_id: "scope-1" });
document.getElementById("run").addEventListener("click", async () => {
  const response = await send("run");
  if (response) startPolling();
});
document.getElementById("pause").addEventListener("click", async () => {
  stopPolling();
  await send("pause");
});
bind("step", "step", { dt_s: 0.000005, steps: 100 });
document.getElementById("reset").addEventListener("click", async () => {
  stopPolling();
  await send("reset");
});
bind("snapshot", "snapshot");
document.getElementById("save-file").addEventListener("click", () =>
  fileCommand("/api/circuit/save", els.filename.value.trim())
);
document.getElementById("load-file").addEventListener("click", async () => {
  stopPolling();
  const result = await fileCommand("/api/circuit/load", els.filename.value.trim());
  if (!result) return;
  if (typeof result.revision === "number") lastRevision = result.revision;
  lastSequence = -1;
  send("snapshot");
});
document.getElementById("refresh-files").addEventListener("click", refreshFiles);
els.saved.addEventListener("change", () => {
  if (els.saved.value) els.filename.value = els.saved.value;
});

async function health() {
  try {
    const result = await fetch("/api/health");
    const data = await result.json();
    els.status.textContent = result.ok ? "connected" : "error";
    if (data.engine_version) {
      els.response.textContent = `engine ${data.engine_version}, protocol ${data.protocol}`;
    }
  } catch (error) {
    els.status.textContent = "disconnected";
  }
}

health();
