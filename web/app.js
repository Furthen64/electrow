import { Transport } from "./js/transport.js";
import { CircuitDocument, snap, TERMINALS } from "./js/circuit.js";
import { SchematicView } from "./js/renderer.js";
import { Scope } from "./js/scope.js";
import { Editor } from "./js/editor.js";

const transport = new Transport();
const documentModel = new CircuitDocument();
const scope = new Scope(document.getElementById("scope"));
const view = new SchematicView(document.getElementById("schematic"));
const editor = new Editor({
  transport,
  view,
  document: documentModel,
  scope,
  onStatus: (text) => {
    document.getElementById("editor-status").textContent = text;
  },
  onDirty: (dirty = true) => setDirty(dirty),
  onSave: () => saveFile(),
  onOpen: () => loadFile(),
  onNew: () => newCircuit(),
});

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

function setBadge(element, text, kind) {
  element.textContent = text;
  element.className = `badge badge-${kind}`;
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

transport.onSnapshot((snapshot) => {
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
    (snapshot.nodes || []).map((node) => [node.id, `${Number(node.voltage_v).toPrecision(6)} V`]),
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
});

transport.onSessionLost = async () => {
  setBadge(els.status, "session restarted", "bad");
  await editor.newCircuit();
};

transport.onStateChange = ({ polling }) => {
  document.getElementById("btn-run").classList.toggle("running", polling);
};

async function showResult(promiseOrResult) {
  const result = await promiseOrResult;
  if (!result.ok) {
    const error = result.error || { code: "error", message: "unknown" };
    els.response.textContent = `${error.code}: ${error.message}`;
    setBadge(els.status, "error", "bad");
    return null;
  }
  els.response.textContent = JSON.stringify(result.result, null, 2);
  setBadge(els.status, "connected", "good");
  return result.result;
}

function bind(id, handler) {
  document.getElementById(id)?.addEventListener("click", handler);
}

async function buildDividerDemo() {
  await editor.newCircuit();
  const ground = documentModel.add("ground", 220, 300, 220, 300);
  const source = documentModel.add("voltage_source", 220, 100, 220, 300);
  const upper = documentModel.add("resistor", 220, 100, 420, 100);
  const lower = documentModel.add("resistor", 420, 100, 420, 300);
  documentModel.add("wire", 420, 300, 220, 300);
  documentModel.setParam(source, "voltage_v", 10);
  documentModel.setParam(upper, "resistance_ohm", 1000);
  documentModel.setParam(lower, "resistance_ohm", 1000);
  await editor.sync();
  await transport.send("step", { dt_s: 0.000005, steps: 1 });
  editor.status("Divider ready — vout should read 5 V");
  document.getElementById("signal-id").value = lower;
}

bind("btn-hello", async () => {
  const health = await transport.health();
  if (health.ok) {
    els.engineInfo.textContent = health.data.engine_version ? `engine ${health.data.engine_version}` : "engine: unknown";
    setBadge(els.status, "connected", "good");
    els.response.textContent = JSON.stringify(health.data, null, 2);
  } else {
    setBadge(els.status, "disconnected", "bad");
    els.response.textContent = String(health.error);
  }
});
bind("btn-new", () => newCircuit());
bind("btn-divider", buildDividerDemo);

bind("btn-add-element", async () => {
  const kind = document.getElementById("build-kind").value;
  const nodeA = document.getElementById("build-node-a").value.trim();
  const nodeB = document.getElementById("build-node-b").value.trim();
  const id = documentModel.add(
    kind,
    snap(view.camera.x),
    snap(view.camera.y),
    snap(view.camera.x + 100),
    snap(view.camera.y),
    document.getElementById("build-id").value.trim() || null
  );
  const element = documentModel.find(id);
  if (TERMINALS[kind] === 2) element.nodes = [nodeA, nodeB];
  await editor.sync();
  editor.status(`Added ${kind} ${id}`);
  document.getElementById("param-id").value = id;
});

bind("btn-set-param", async () => {
  const id = document.getElementById("param-id").value.trim();
  const key = document.getElementById("param-key").value;
  const value = Number(document.getElementById("param-value").value);
  if (documentModel.find(id)) {
    documentModel.setParam(id, key, value);
    await editor.sync();
  } else {
    await showResult(transport.send("parameter_edit", {
      base_revision: transport.revision,
      element_id: id,
      params: { [key]: value },
    }));
  }
});

bind("btn-subscribe", async () =>
  await showResult(transport.send("subscribe", {
    stream_id: "scope-1",
    signals: [
      {
        id: "signal",
        element_id: document.getElementById("signal-id").value.trim(),
        quantity: document.getElementById("signal-quantity").value,
        unit: document.getElementById("signal-quantity").value === "current" ? "A" : "V",
      },
    ],
  }))
);
bind("btn-unsubscribe", async () =>
  await showResult(transport.send("unsubscribe", { stream_id: "scope-1" }))
);

bind("btn-run", async () => {
  const result = await transport.send("run");
  if (result.ok) transport.setPolling(true);
  else await showResult(result);
});
bind("btn-pause", async () => {
  transport.setPolling(false);
  await showResult(transport.send("pause"));
});
bind("btn-step", async () =>
  await showResult(transport.send("step", {
    dt_s: Number(document.getElementById("step-dt").value),
    steps: Number(document.getElementById("step-count").value),
  }))
);
bind("btn-reset", async () => {
  transport.setPolling(false);
  await showResult(transport.send("reset"));
});
bind("btn-snapshot", async () => await showResult(transport.send("snapshot")));

bind("btn-load-dump", async () => {
  const dump = document.getElementById("dump-input").value;
  const selected = document.getElementById("dump-format").value;
  const format = selected !== "auto" ? selected : dump.trimStart().startsWith("<") ? "xml" : "text";
  transport.setPolling(false);
  const result = await showResult(transport.send("load_circuit", { dump, format, base_revision: transport.revision }));
  if (result) await editor.pullFromBackend();
});
bind("btn-export", async () => {
  const result = await showResult(transport.send("export_circuit", {}));
  if (!result) return;
  els.exportOutput.hidden = false;
  els.exportOutput.textContent = JSON.stringify(result, null, 2);
});

function setDirty(dirty = true) {
  const badge = document.getElementById("dirty");
  badge.hidden = !dirty;
  badge.className = dirty ? "badge badge-warn" : "badge badge-muted";
}

async function saveFile() {
  const name = document.getElementById("file-name").value.trim();
  if (!name) {
    await showResult({ ok: false, error: { code: "bad_request", message: "Enter a name first" } });
    return false;
  }
  const result = await showResult(transport.fileCommand("/api/circuit/save", name));
  if (result) setDirty(false);
  return Boolean(result);
}

async function loadFile() {
  const name = document.getElementById("file-name").value.trim();
  if (!name) {
    await showResult({ ok: false, error: { code: "bad_request", message: "Enter a name first" } });
    return false;
  }
  transport.setPolling(false);
  const result = await showResult(transport.fileCommand("/api/circuit/load", name));
  if (!result) return false;
  await editor.pullFromBackend();
  setDirty(false);
  return true;
}

async function newCircuit() {
  await editor.newCircuit();
  setDirty(false);
}

bind("btn-save-file", () => saveFile());
bind("btn-load-file", () => loadFile());
bind("btn-apply-preset", async () => {
  const id = document.getElementById("param-id").value.trim();
  const name = document.getElementById("transistor-preset").value;
  if (!name) return showResult({ ok: false, error: { code: "bad_request", message: "Choose a preset" } });
  const ok = await editor.applyPreset(id, name);
  if (!ok) showResult({ ok: false, error: { code: "bad_request", message: "Select a bjt element" } });
});
bind("btn-refresh-files", async () => {
  const select = document.getElementById("file-list");
  select.replaceChildren(new Option("choose…", ""));
  for (const name of await transport.listFiles()) select.add(new Option(name, name));
});
document.getElementById("file-list").addEventListener("change", (event) => {
  if (event.target.value) document.getElementById("file-name").value = event.target.value;
});

async function health() {
  const result = await transport.health();
  if (result.ok) {
    setBadge(els.status, "connected", "good");
    els.engineInfo.textContent = result.data.engine_version ? `engine ${result.data.engine_version}` : "engine: unknown";
  } else {
    setBadge(els.status, "disconnected", "bad");
  }
}

health();
document.getElementById("btn-refresh-files").click();
