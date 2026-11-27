export const GRID = 20;

export function snap(value) {
  return Math.round(value / GRID) * GRID;
}

export function nodeId(x, y) {
  return `n${snap(x)}_${snap(y)}`;
}

export const TERMINALS = {
  wire: 2,
  ground: 1,
  resistor: 2,
  voltage_source: 2,
  current_source: 2,
  capacitor: 2,
  inductor: 2,
  diode: 2,
  switch: 2,
};

export const DEFAULT_PARAMS = {
  wire: {},
  ground: {},
  resistor: { resistance_ohm: 1000 },
  voltage_source: { voltage_v: 5 },
  current_source: { current_a: 0.001 },
  capacitor: { capacitance_f: 1e-6 },
  inductor: { inductance_h: 1e-3 },
  diode: { is_a: 1e-12, emission: 1 },
  switch: { closed: 1, resistance_ohm: 1e-3 },
  bjt: { beta: 100, is_a: 1e-12, emission: 1, pnp: 0 },
  mosfet: { pnp: 0, vt: 1.5, beta: 0.02 },
  vactrol: {
    dark_resistance_ohm: 10e6,
    resistance_at_1ma_ohm: 30e3,
    response_exponent: 0.8,
    minimum_resistance_ohm: 1.5e3,
    turn_on_s: 2.5e-3,
    turn_off_s: 8.403422185645153e-3,
  },
};

function clone(elements) {
  return JSON.parse(JSON.stringify(elements));
}

export class CircuitDocument {
  constructor() {
    this.elements = [];
    this.synced = [];
    this.undoStack = [];
    this.redoStack = [];
    this.nextId = 1;
  }

  find(id) {
    return this.elements.find((element) => element.id === id);
  }

  nodesFor(kind, x, y, x2, y2) {
    if (kind === "ground") return [nodeId(x, y)];
    const first = nodeId(x, y);
    const second = nodeId(x2, y2);
    if (kind === "bjt" || kind === "mosfet") {
      return [first, second, nodeId(x2, y2 + GRID * 2)];
    }
    if (kind === "vactrol") {
      return [
        first,
        nodeId(x, y + GRID * 2),
        second,
        nodeId(x2, y2 + GRID * 2),
      ];
    }
    return [first, second];
  }

  commit(change) {
    this.undoStack.push(clone(this.elements));
    if (this.undoStack.length > 60) this.undoStack.shift();
    this.redoStack.length = 0;
    change(this.elements);
    return true;
  }

  add(kind, x, y, x2 = x + GRID * 4, y2 = y, requestedId = null) {
    const generated = `d${this.nextId++}`;
    const id = requestedId || generated;
    if (this.find(id)) return null;
    this.commit((elements) => {
      elements.push({
        id,
        kind,
        x: snap(x),
        y: snap(y),
        x2: snap(x2),
        y2: snap(y2),
        nodes: this.nodesFor(kind, x, y, x2, y2),
        params: { ...DEFAULT_PARAMS[kind] },
      });
    });
    return id;
  }

  move(id, dx, dy) {
    const before = this.find(id);
    if (!before) return;
    const nx = snap(before.x + dx);
    const ny = snap(before.y + dy);
    const nx2 = snap(before.x2 + dx);
    const ny2 = snap(before.y2 + dy);
    if (nx === before.x && ny === before.y && nx2 === before.x2 && ny2 === before.y2) return;
    this.commit((elements) => {
      const element = elements.find((candidate) => candidate.id === id);
      element.x = nx;
      element.y = ny;
      element.x2 = nx2;
      element.y2 = ny2;
      element.nodes = this.nodesFor(element.kind, nx, ny, nx2, ny2);
    });
  }

  remove(id) {
    if (!this.find(id)) return;
    this.commit((elements) => {
      const index = elements.findIndex((element) => element.id === id);
      elements.splice(index, 1);
    });
  }

  setParam(id, key, value) {
    const element = this.find(id);
    if (!element || element.params[key] === value) return;
    this.commit((elements) => {
      elements.find((candidate) => candidate.id === id).params[key] = value;
    });
  }

  undo() {
    if (!this.undoStack.length) return false;
    this.redoStack.push(clone(this.elements));
    this.elements = this.undoStack.pop();
    return true;
  }

  redo() {
    if (!this.redoStack.length) return false;
    this.undoStack.push(clone(this.elements));
    this.elements = this.redoStack.pop();
    return true;
  }

  loadElements(elements) {
    this.commit((current) => {
      current.length = 0;
      for (const element of elements) current.push(clone([element])[0]);
      this.nextId = Math.max(this.nextId, elements.length + 1);
    });
  }

  async sync(transport) {
    const before = new Map(this.synced.map((element) => [element.id, element]));
    const after = new Map(this.elements.map((element) => [element.id, element]));
    const ops = [];
    for (const element of this.synced) {
      if (!after.has(element.id)) ops.push({ op: "remove", element_id: element.id });
    }
    for (const element of this.elements) {
      const previous = before.get(element.id);
      if (!previous) {
        ops.push({
          op: "add",
          element_id: element.id,
          kind: element.kind,
          node_ids: [...element.nodes],
        });
      } else if (JSON.stringify(previous.nodes) !== JSON.stringify(element.nodes)) {
        ops.push({ op: "connect", element_id: element.id, node_ids: [...element.nodes] });
      }
    }
    if (ops.length) {
      const result = await transport.send("topology_edit", {
        base_revision: transport.revision,
        ops,
      });
      if (!result.ok) return result;
    }
    for (const element of this.elements) {
      const previous = before.get(element.id);
      for (const [key, value] of Object.entries(element.params)) {
        if (previous && previous.params[key] === value) continue;
        const result = await transport.send("parameter_edit", {
          base_revision: transport.revision,
          element_id: element.id,
          params: { [key]: value },
        });
        if (!result.ok) return result;
      }
    }
    this.synced = clone(this.elements);
    return { ok: true };
  }

  resetSynced() {
    this.synced = clone(this.elements);
    this.undoStack.length = 0;
    this.redoStack.length = 0;
  }
}
