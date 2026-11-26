import { GRID, snap } from "./circuit.js";

const PALETTE = ["wire", "resistor", "voltage_source", "current_source", "capacitor", "inductor", "diode", "switch", "ground"];

export class Editor {
  constructor({ transport, view, document, scope, onStatus }) {
    this.transport = transport;
    this.view = view;
    this.document = document;
    this.scope = scope;
    this.onStatus = onStatus || (() => {});
    this.selected = null;
    this.busy = false;

    view.setDocument(document);
    view.callbacks.onSelect = (id) => {
      this.selected = id;
      this.onStatus(id ? `Selected ${id}` : "Nothing selected");
    };
    view.callbacks.onMove = (id, dx, dy) => {
      this.document.move(id, dx, dy);
      this.sync();
    };
    view.callbacks.onWire = (start, end) => {
      this.document.add("wire", start.x, start.y, end.x, end.y);
      this.sync();
      this.onStatus("Wire added");
    };

    this.transport.onSnapshot((snapshot) => this.onSnapshot(snapshot));
    this.bindControls();
    this.bindKeys();
    this.setTool("select");
  }

  status(text) {
    this.onStatus(text);
  }

  setTool(tool) {
    this.view.setTool(tool);
    for (const name of ["select", "wire", "pan"]) {
      document.getElementById(`tool-${name}`)?.classList.toggle("active", tool === name);
    }
    this.status(tool === "wire" ? "Wire: click two endpoints (Esc to cancel)" :
      tool === "pan" ? "Pan: drag to move the viewport" : "Select: click to select, drag to move");
  }

  bindControls() {
    for (const name of ["select", "wire", "pan"]) {
      document.getElementById(`tool-${name}`)?.addEventListener("click", () => this.setTool(name));
    }
    const palette = document.getElementById("palette");
    if (palette) {
      for (const kind of PALETTE) {
        const button = document.createElement("button");
        button.type = "button";
        button.textContent = kind.replace("_", " ");
        button.addEventListener("click", () => this.place(kind));
        palette.append(button);
      }
    }
    document.getElementById("btn-undo")?.addEventListener("click", () => this.undo());
    document.getElementById("btn-redo")?.addEventListener("click", () => this.redo());
    document.getElementById("btn-delete")?.addEventListener("click", () => this.deleteSelected());
    document.getElementById("btn-fit")?.addEventListener("click", () => this.view.resetView());
  }

  bindKeys() {
    window.addEventListener("keydown", (event) => {
      const tag = event.target?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
      if (event.key === "Delete" || event.key === "Backspace") {
        this.deleteSelected();
        event.preventDefault();
      } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") {
        if (event.shiftKey) this.redo();
        else this.undo();
        event.preventDefault();
      } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "y") {
        this.redo();
        event.preventDefault();
      } else if (event.key === "Escape") {
        this.setTool("select");
      } else if (event.code === "Space") {
        this.view.setPanMode(true);
        event.preventDefault();
      }
    });
    window.addEventListener("keyup", (event) => {
      if (event.code === "Space") this.view.setPanMode(false);
    });
  }

  place(kind) {
    const { x, y } = this.view.camera;
    const id = this.document.add(kind, snap(x), snap(y), snap(x + GRID * 5), snap(y));
    this.selected = id;
    this.view.selected = id;
    this.sync();
    this.status(`Added ${kind} ${id}`);
  }

  deleteSelected() {
    if (!this.selected) return;
    const id = this.selected;
    this.document.remove(id);
    this.selected = null;
    this.view.selected = null;
    this.sync();
    this.status(`Deleted ${id}`);
  }

  undo() {
    if (!this.document.undo()) return;
    this.sync();
    this.status("Undo");
  }

  redo() {
    if (!this.document.redo()) return;
    this.sync();
    this.status("Redo");
  }

  async sync() {
    if (this.busy) return;
    this.busy = true;
    try {
      const result = await this.document.sync(this.transport);
      if (!result.ok) this.status(`Sync failed: ${result.error?.message || "unknown"}`);
      await this.subscribeSignals();
    } finally {
      this.busy = false;
    }
  }

  async subscribeSignals() {
    const signals = [];
    for (const element of this.document.elements) {
      if (signals.length >= 60) break;
      signals.push({
        id: `i:${element.id}`,
        element_id: element.id,
        quantity: "current",
        unit: "A",
      });
      if (element.kind !== "ground" && signals.length < 60) {
        signals.push({
          id: `v:${element.id}`,
          element_id: element.id,
          quantity: "voltage",
          unit: "V",
        });
      }
    }
    if (!signals.length) return;
    await this.transport.send("subscribe", { stream_id: "schematic", signals }, { silent: true });
    this.scope.setSignals(signals);
  }

  onSnapshot(snapshot) {
    this.view.setSnapshot(snapshot);
    const values = {};
    for (const measurement of snapshot.measurements || []) {
      values[measurement.id] = measurement.value;
    }
    this.scope.addSample(snapshot.simulation_time_s, values);
    this.scope.draw();
  }

  async newCircuit() {
    this.transport.setPolling(false);
    await this.transport.send("create", { circuit_id: "schematic" });
    this.document.loadElements([]);
    this.document.resetSynced();
    this.selected = null;
    this.view.selected = null;
    this.scope.clear();
    this.status("New empty circuit");
  }

  async loadFromDocument(elements) {
    this.document.loadElements(elements);
    this.document.resetSynced();
    this.selected = null;
    this.view.selected = null;
    this.scope.clear();
    await this.sync();
    this.status(`Loaded ${elements.length} elements`);
  }

  autoLayout(count) {
    const result = [];
    for (let i = 0; i < count; i += 1) {
      const column = i % 4;
      const row = Math.floor(i / 4);
      result.push({
        id: `e${i + 1}`,
        x: snap(100 + column * 160),
        y: snap(100 + row * 100),
        x2: snap(180 + column * 160),
        y2: snap(100 + row * 100),
      });
    }
    return result;
  }

  async pullFromBackend() {
    const response = await this.transport.send("export_circuit", {});
    if (!response.ok) return null;
    const source = response.result;
    const layout = this.autoLayout((source.elements || []).length);
    const elements = (source.elements || []).map((element, index) => ({
      id: element.id,
      kind: element.kind,
      x: layout[index].x,
      y: layout[index].y,
      x2: layout[index].x2,
      y2: layout[index].y2,
      nodes: [...element.nodes],
      params: { ...(element.params || {}) },
    }));
    await this.loadFromDocument(elements);
    return elements.length;
  }
}
