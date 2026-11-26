import { GRID, snap, nodeId } from "./circuit.js";
import { drawSymbol, hitTest, COLORS } from "./symbols.js";

export class SchematicView {
  constructor(canvas, callbacks = {}) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d");
    this.callbacks = callbacks;
    this.document = null;
    this.snapshot = null;
    this.tool = "select";
    this.camera = { x: 200, y: 160 };
    this.zoom = 1;
    this.selected = null;
    this.hover = null;
    this.pointer = null;
    this.wireStart = null;
    this.preview = null;
    this.phase = 0;
    this.lastFrame = 0;
    this.panMode = false;
    this.dpr = 1;

    canvas.addEventListener("pointerdown", (event) => this.onDown(event));
    canvas.addEventListener("pointermove", (event) => this.onMove(event));
    canvas.addEventListener("pointerup", (event) => this.onUp(event));
    canvas.addEventListener("pointercancel", (event) => this.onUp(event));
    canvas.addEventListener("wheel", (event) => this.onWheel(event), { passive: false });
    canvas.addEventListener("contextmenu", (event) => event.preventDefault());
    this.resize();
    new ResizeObserver(() => this.resize()).observe(canvas);
    requestAnimationFrame((time) => this.frame(time));
  }

  setDocument(document) {
    this.document = document;
  }

  setSnapshot(snapshot) {
    this.snapshot = snapshot;
  }

  setTool(tool) {
    this.tool = tool;
    this.wireStart = null;
    this.canvas.style.cursor = tool === "pan" ? "grab" : tool === "wire" ? "crosshair" : "default";
  }

  setPanMode(enabled) {
    this.panMode = enabled;
    this.canvas.style.cursor = enabled ? "grab" : this.tool === "wire" ? "crosshair" : "default";
  }

  resize() {
    const rect = this.canvas.getBoundingClientRect();
    this.dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.max(1, Math.round(rect.width * this.dpr));
    this.canvas.height = Math.max(1, Math.round(rect.height * this.dpr));
  }

  toWorld(clientX, clientY) {
    const rect = this.canvas.getBoundingClientRect();
    return {
      x: (clientX - rect.left - rect.width / 2) / this.zoom + this.camera.x,
      y: (clientY - rect.top - rect.height / 2) / this.zoom + this.camera.y,
    };
  }

  toScreen(x, y) {
    const rect = this.canvas.getBoundingClientRect();
    return {
      x: (x - this.camera.x) * this.zoom + rect.width / 2,
      y: (y - this.camera.y) * this.zoom + rect.height / 2,
    };
  }

  onDown(event) {
    this.canvas.setPointerCapture(event.pointerId);
    const world = this.toWorld(event.clientX, event.clientY);
    if (event.button === 1 || event.button === 2 || this.tool === "pan" || this.panMode) {
      this.pointer = { mode: "pan", x: event.clientX, y: event.clientY, camera: { ...this.camera } };
      return;
    }
    if (this.tool === "wire") {
      this.pointerWorld = world;
      const point = { x: snap(world.x), y: snap(world.y) };
      if (!this.wireStart) {
        this.wireStart = point;
      } else {
        this.callbacks.onWire?.(this.wireStart, point);
        this.wireStart = null;
      }
      return;
    }
    const hit = this.findHit(world);
    this.selected = hit ? hit.id : null;
    this.callbacks.onSelect?.(this.selected);
    if (hit) {
      this.pointer = {
        mode: "drag",
        id: hit.id,
        start: world,
        original: { x: hit.x, y: hit.y, x2: hit.x2, y2: hit.y2 },
      };
      this.preview = { id: hit.id, dx: 0, dy: 0 };
    }
  }

  onMove(event) {
    const world = this.toWorld(event.clientX, event.clientY);
    this.pointerWorld = world;
    this.hover = this.findHit(world)?.id || null;
    if (!this.pointer) return;
    if (this.pointer.mode === "pan") {
      this.camera.x = this.pointer.camera.x - (event.clientX - this.pointer.x) / this.zoom;
      this.camera.y = this.pointer.camera.y - (event.clientY - this.pointer.y) / this.zoom;
    } else if (this.pointer.mode === "drag") {
      this.preview = {
        id: this.pointer.id,
        dx: snap(world.x - this.pointer.start.x),
        dy: snap(world.y - this.pointer.start.y),
      };
    }
  }

  onUp(event) {
    if (this.canvas.hasPointerCapture(event.pointerId)) {
      this.canvas.releasePointerCapture(event.pointerId);
    }
    if (this.pointer?.mode === "drag" && this.preview) {
      const { dx, dy } = this.preview;
      if (dx || dy) this.callbacks.onMove?.(this.pointer.id, dx, dy);
    }
    this.pointer = null;
    this.preview = null;
  }

  onWheel(event) {
    event.preventDefault();
    const before = this.toWorld(event.clientX, event.clientY);
    this.zoom = Math.max(0.35, Math.min(3, this.zoom * (event.deltaY < 0 ? 1.12 : 0.89)));
    const after = this.toWorld(event.clientX, event.clientY);
    this.camera.x += before.x - after.x;
    this.camera.y += before.y - after.y;
  }

  findHit(world) {
    if (!this.document) return null;
    for (let index = this.document.elements.length - 1; index >= 0; index -= 1) {
      const element = this.document.elements[index];
      if (hitTest(element, world.x, world.y)) return element;
    }
    return null;
  }

  frame(time) {
    const dt = Math.min(0.05, (time - this.lastFrame) / 1000 || 0);
    this.lastFrame = time;
    this.phase += dt;
    this.draw();
    requestAnimationFrame((next) => this.frame(next));
  }

  draw() {
    const ctx = this.ctx;
    const rect = this.canvas.getBoundingClientRect();
    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    ctx.clearRect(0, 0, rect.width, rect.height);
    ctx.save();
    ctx.translate(
      rect.width / 2 - this.camera.x * this.zoom,
      rect.height / 2 - this.camera.y * this.zoom
    );
    ctx.scale(this.zoom, this.zoom);
    this.drawGrid(ctx, rect);
    if (!this.document) {
      ctx.restore();
      return;
    }

    const voltages = new Map((this.snapshot?.nodes || []).map((node) => [node.id, node.voltage_v]));
    const currents = new Map(
      (this.snapshot?.measurements || []).map((measurement) => [measurement.id, measurement.value])
    );

    for (const element of this.document.elements) {
      const preview = this.preview?.id === element.id ? this.preview : null;
      const shifted = preview
        ? { ...element, x: element.x + preview.dx, y: element.y + preview.dy,
            x2: element.x2 + preview.dx, y2: element.y2 + preview.dy }
        : element;
      drawSymbol(ctx, shifted, { selected: this.selected === element.id });
      this.drawTerminals(ctx, shifted, voltages);
      this.drawCurrent(ctx, shifted, currents.get(`i:${element.id}`) || 0);
    }

    if (this.wireStart) {
      const end = this.pointerWorld || this.wireStart;
      ctx.save();
      ctx.setLineDash([6 / this.zoom, 5 / this.zoom]);
      ctx.strokeStyle = COLORS.wire;
      ctx.lineWidth = 2 / this.zoom;
      ctx.beginPath();
      ctx.moveTo(this.wireStart.x, this.wireStart.y);
      ctx.lineTo(end.x, end.y);
      ctx.stroke();
      ctx.restore();
    }
    ctx.restore();
  }

  drawGrid(ctx, rect) {
    if (GRID * this.zoom < 6) return;
    const left = this.camera.x - rect.width / (2 * this.zoom);
    const top = this.camera.y - rect.height / (2 * this.zoom);
    const right = this.camera.x + rect.width / (2 * this.zoom);
    const bottom = this.camera.y + rect.height / (2 * this.zoom);
    ctx.lineWidth = 1 / this.zoom;
    ctx.strokeStyle = "#202a36";
    ctx.beginPath();
    for (let x = Math.ceil(left / GRID) * GRID; x <= right; x += GRID) {
      ctx.moveTo(x, top);
      ctx.lineTo(x, bottom);
    }
    for (let y = Math.ceil(top / GRID) * GRID; y <= bottom; y += GRID) {
      ctx.moveTo(left, y);
      ctx.lineTo(right, y);
    }
    ctx.stroke();
  }

  drawTerminals(ctx, element, voltages) {
    const points = element.kind === "ground"
      ? [[element.x, element.y]]
      : [[element.x, element.y], [element.x2, element.y2]];
    element.nodes.forEach((node, index) => {
      if (!points[index]) return;
      const [x, y] = points[index];
      ctx.beginPath();
      ctx.arc(x, y, 3.5 / this.zoom, 0, Math.PI * 2);
      ctx.fillStyle = "#dbe6f4";
      ctx.fill();
      const voltage = voltages.get(node);
      if (voltage !== undefined && Math.abs(voltage) > 1e-9) {
        ctx.fillStyle = "#7f8fa4";
        ctx.font = `${10 / this.zoom}px ui-monospace, monospace`;
        ctx.textAlign = "left";
        ctx.fillText(`${voltage.toFixed(2)}V`, x + 6 / this.zoom, y - 8 / this.zoom);
      }
    });
  }

  drawCurrent(ctx, element, current) {
    const magnitude = Math.abs(current);
    if (magnitude < 1e-9 || element.kind === "ground") return;
    const speed = Math.min(2, 0.2 + magnitude * 200);
    const phase = (this.phase * speed) % 1;
    const direction = Math.sign(current);
    for (let i = 0; i < 3; i += 1) {
      let t = (phase + i / 3) % 1;
      if (direction < 0) t = 1 - t;
      const x = element.x + (element.x2 - element.x) * t;
      const y = element.y + (element.y2 - element.y) * t;
      ctx.beginPath();
      ctx.arc(x, y, 2.6 / this.zoom, 0, Math.PI * 2);
      ctx.fillStyle = "#6ee7ff";
      ctx.fill();
    }
  }

  resetView() {
    this.camera = { x: 200, y: 160 };
    this.zoom = 1;
    this.selected = null;
  }
}
