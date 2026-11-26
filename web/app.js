const els = {
  status: document.getElementById("status"),
  engineInfo: document.getElementById("engine-info"),
  response: document.getElementById("response"),
  circuit: document.getElementById("stat-circuit"),
  revision: document.getElementById("stat-revision"),
  sequence: document.getElementById("stat-sequence"),
  time: document.getElementById("stat-time"),
  dt: document.getElementById("stat-dt"),
  simStatus: document.getElementById("stat-status"),
  measurements: document.getElementById("measurements-body"),
  nodes: document.getElementById("nodes-body"),
  diagnostics: document.getElementById("diagnostics-list"),
  exportOutput: document.getElementById("export-output"),
};

let nextId = 1;
let lastRevision = 0;
let lastSequence = -1;
let pollTimer = null;

function setBadge(element, text, kind) {
  element.textContent = text;
  element.className = `badge badge-${kind}`;
}

function startPolling() {
  if (pollTimer !== null) return;
  document.getElementById("btn-run").classList.add("running");
  pollTimer = setInterval(() => send("snapshot", {}, true), 50);
}

function stopPolling() {
  if (pollTimer !== null) clearInterval(pollTimer);
  pollTimer = null;
  document.getElementById("btn-run").classList.remove("running");
}

function renderRows(target, rows, emptyText) {
  target.replaceChildren();
  if (!rows.length) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 4;
    td.className = "muted";
    td.textContent = emptyText;
    tr.append(td);
    target.append(tr);
    return;
  }
  for (const cells of rows) {
    const tr = document.createElement("tr");
    for (const cell of cells) {
      const td = document.createElement("td");
      td.textContent = String(cell);
      tr.append(td);
    }
    target.append(tr);
  }
}

function renderSnapshot(snapshot) {
  els.circuit.textContent = snapshot.circuit_id;
  els.revision.textContent = String(snapshot.revision);
  els.sequence.textContent = String(snapshot.sequence);
  els.time.textContent = `${Number(snapshot.simulation_time_s).toPrecision(4)} s`;
  els.dt.textContent = `${Number(snapshot.time_step_s).toPrecision(3)} s`;
  const state = snapshot.status || "unknown";
  const kind = state === "failed" ? "bad" : state === "running" ? "good" : "muted";
  setBadge(els.simStatus, `${state}${snapshot.running ? " · running" : ""}`, kind);

  renderRows(
    els.measurements,
    (snapshot.measurements || []).map((m) => [
      m.id,
      m.element_id,
      m.quantity,
      `${Number(m.value).toPrecision(6)} ${m.unit}`,
    ]),
    "No subscriptions yet."
  );
  renderRows(
    els.nodes,
    (snapshot.nodes || []).map((n) => [n.id, `${Number(n.voltage_v).toPrecision(6)} V`]),
    "No nodes yet."
  );

  els.diagnostics.replaceChildren();
  const items = snapshot.diagnostics || [];
  if (!items.length) {
    const li = document.createElement("li");
    li.className = "muted empty";
    li.textContent = "None — circuit looks healthy.";
    els.diagnostics.append(li);
  } else {
    for (const item of items) {
      const li = document.createElement("li");
      li.textContent =
        `${item.kind}` +
        (item.node_id ? ` · node ${item.node_id}` : "") +
        (item.element_id ? ` · ${item.element_id}` : "") +
        (item.parameter ? ` · ${item.parameter}` : "") +
        (item.change_v !== undefined ? ` · ΔV ${Number(item.change_v).toPrecision(3)}` : "");
      els.diagnostics.append(li);
    }
  }
}

function showError(error) {
  els.response.textContent = `${error.code || "error"}: ${error.message || error}`;
  setBadge(els.status, "error", "bad");
}

async function send(cmd, args = {}, silent = false) {
  const request = { v: 1, id: `web-${nextId++}`, cmd, args };
  try {
    const result = await fetch("/api/command", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
    const data = await result.json();
    if (!result.ok || data.ok === false) {
      showError(data.error || { code: "http", message: `HTTP ${result.status}` });
      if (data.error && data.error.code === "session_lost") {
        stopPolling();
        lastRevision = 0;
        lastSequence = -1;
        send("create");
      }
      return null;
    }
    const response = data.result;
    if (typeof response.revision === "number") lastRevision = response.revision;
    if (!silent) els.response.textContent = JSON.stringify(response, null, 2);
    setBadge(els.status, "connected", "good");
    if (response && response.type === "snapshot") {
      if (response.sequence <= lastSequence) {
        setBadge(els.status, "stale frame dropped", "warn");
        return response;
      }
      lastSequence = response.sequence;
      renderSnapshot(response);
    } else {
      send("snapshot", {}, true);
    }
    return response;
  } catch (error) {
    els.response.textContent = String(error);
    setBadge(els.status, "disconnected", "bad");
    return null;
  }
}

async function fileCommand(path, name) {
  if (!name) {
    showError({ code: "bad_request", message: "Enter a circuit name first." });
    return null;
  }
  try {
    const result = await fetch(path, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ v: 1, id: `web-${nextId++}`, name }),
    });
    const data = await result.json();
    if (!result.ok || data.ok === false) {
      showError(data.error || { code: "http", message: `HTTP ${result.status}` });
      return null;
    }
    els.response.textContent = JSON.stringify(data.result, null, 2);
    setBadge(els.status, "connected", "good");
    return data.result;
  } catch (error) {
    els.response.textContent = String(error);
    setBadge(els.status, "disconnected", "bad");
    return null;
  }
}

async function refreshFiles() {
  try {
    const data = await (await fetch("/api/circuits")).json();
    const select = document.getElementById("file-list");
    select.replaceChildren(new Option("choose…", ""));
    for (const name of data.names || []) select.add(new Option(name, name));
  } catch (error) {
    els.response.textContent = String(error);
  }
}

async function createDivider() {
  await send("create", { circuit_id: "divider" });
  await send("topology_edit", {
    base_revision: lastRevision,
    ops: [
      { op: "add", element_id: "gnd", kind: "ground", node_ids: ["gnd"] },
      { op: "add", element_id: "v1", kind: "voltage_source", node_ids: ["n1", "gnd"] },
      { op: "add", element_id: "r1", kind: "resistor", node_ids: ["n1", "n2"] },
      { op: "add", element_id: "r2", kind: "resistor", node_ids: ["n2", "gnd"] },
    ],
  });
  for (const [element_id, params] of [
    ["v1", { voltage_v: 10 }],
    ["r1", { resistance_ohm: 1000 }],
    ["r2", { resistance_ohm: 1000 }],
  ]) {
    await send("parameter_edit", { base_revision: lastRevision, element_id, params });
  }
  await send("subscribe", {
    stream_id: "scope-1",
    signals: [{ id: "vout", element_id: "r2", quantity: "voltage", unit: "V" }],
  });
  await send("step", { dt_s: 0.000005, steps: 1 });
  document.getElementById("signal-id").value = "r2";
}

function bind(id, handler) {
  document.getElementById(id).addEventListener("click", handler);
}

bind("btn-hello", () => send("hello"));
bind("btn-new", () => {
  stopPolling();
  send("create");
});
bind("btn-divider", () => {
  stopPolling();
  createDivider();
});
bind("btn-add-element", () =>
  send("topology_edit", {
    base_revision: lastRevision,
    ops: [
      {
        op: "add",
        element_id: document.getElementById("build-id").value.trim(),
        kind: document.getElementById("build-kind").value,
        node_ids: [
          document.getElementById("build-node-a").value.trim(),
          document.getElementById("build-node-b").value.trim(),
        ],
      },
    ],
  })
);
bind("btn-set-param", () =>
  send("parameter_edit", {
    base_revision: lastRevision,
    element_id: document.getElementById("param-id").value.trim(),
    params: {
      [document.getElementById("param-key").value]: Number(document.getElementById("param-value").value),
    },
  })
);
bind("btn-subscribe", () =>
  send("subscribe", {
    stream_id: "scope-1",
    signals: [
      {
        id: "signal",
        element_id: document.getElementById("signal-id").value.trim(),
        quantity: document.getElementById("signal-quantity").value,
        unit: document.getElementById("signal-quantity").value === "current" ? "A" : "V",
      },
    ],
  })
);
bind("btn-unsubscribe", () => send("unsubscribe", { stream_id: "scope-1" }));
bind("btn-run", async () => {
  const response = await send("run");
  if (response) startPolling();
});
bind("btn-pause", async () => {
  stopPolling();
  await send("pause");
});
bind("btn-step", () =>
  send("step", {
    dt_s: Number(document.getElementById("step-dt").value),
    steps: Number(document.getElementById("step-count").value),
  })
);
bind("btn-reset", async () => {
  stopPolling();
  await send("reset");
});
bind("btn-snapshot", () => send("snapshot"));
bind("btn-load-dump", () => {
  const dump = document.getElementById("dump-input").value;
  const selected = document.getElementById("dump-format").value;
  const format =
    selected !== "auto" ? selected : dump.trimStart().startsWith("<") ? "xml" : "text";
  stopPolling();
  send("load_circuit", { dump, format, base_revision: lastRevision });
});
bind("btn-export", async () => {
  const result = await send("export_circuit", {});
  if (!result) return;
  els.exportOutput.hidden = false;
  els.exportOutput.textContent = JSON.stringify(result, null, 2);
});

bind("btn-save-file", () =>
  fileCommand("/api/circuit/save", document.getElementById("file-name").value.trim())
);
bind("btn-load-file", async () => {
  stopPolling();
  const result = await fileCommand("/api/circuit/load", document.getElementById("file-name").value.trim());
  if (!result) return;
  if (typeof result.revision === "number") lastRevision = result.revision;
  lastSequence = -1;
  send("snapshot");
});
bind("btn-refresh-files", refreshFiles);
document.getElementById("file-list").addEventListener("change", (event) => {
  if (event.target.value) document.getElementById("file-name").value = event.target.value;
});

async function health() {
  try {
    const result = await fetch("/api/health");
    const data = await result.json();
    setBadge(els.status, result.ok ? "connected" : "error", result.ok ? "good" : "bad");
    els.engineInfo.textContent = data.engine_version ? `engine ${data.engine_version}` : "engine: unknown";
  } catch (error) {
    setBadge(els.status, "disconnected", "bad");
  }
}

refreshFiles();
health();
