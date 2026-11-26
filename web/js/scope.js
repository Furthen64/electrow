const COLORS = ["#6ee7ff", "#f6c86e", "#a8e6a1", "#ff9ecb", "#c3a6ff", "#9fb0c3"];

export class Scope {
  constructor(canvas, capacity = 4096) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d");
    this.capacity = capacity;
    this.signals = new Map();
    this.windowSeconds = 0.01;
    this.dpr = 1;
    this.resize();
    new ResizeObserver(() => this.resize()).observe(canvas);
  }

  setSignals(signals) {
    const ids = new Set(signals.map((signal) => signal.id));
    for (const id of [...this.signals.keys()]) {
      if (!ids.has(id)) this.signals.delete(id);
    }
    for (const signal of signals) {
      if (!this.signals.has(signal.id)) {
        this.signals.set(signal.id, {
          unit: signal.unit,
          times: new Float64Array(this.capacity),
          values: new Float64Array(this.capacity),
          head: 0,
          count: 0,
        });
      }
    }
  }

  addSample(time, values) {
    for (const [id, value] of Object.entries(values)) {
      const signal = this.signals.get(id);
      if (!signal || !Number.isFinite(value)) continue;
      signal.times[signal.head] = time;
      signal.values[signal.head] = value;
      signal.head = (signal.head + 1) % this.capacity;
      signal.count = Math.min(signal.count + 1, this.capacity);
    }
  }

  clear() {
    for (const signal of this.signals.values()) {
      signal.head = 0;
      signal.count = 0;
    }
  }

  resize() {
    const rect = this.canvas.getBoundingClientRect();
    this.dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.max(1, Math.round(rect.width * this.dpr));
    this.canvas.height = Math.max(1, Math.round(rect.height * this.dpr));
  }

  samples(signal) {
    const start = (signal.head - signal.count + this.capacity) % this.capacity;
    const result = [];
    for (let i = 0; i < signal.count; i += 1) {
      const index = (start + i) % this.capacity;
      result.push([signal.times[index], signal.values[index]]);
    }
    return result;
  }

  draw() {
    const ctx = this.ctx;
    const rect = this.canvas.getBoundingClientRect();
    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    ctx.clearRect(0, 0, rect.width, rect.height);
    ctx.fillStyle = "#0a0e13";
    ctx.fillRect(0, 0, rect.width, rect.height);
    const left = 44;
    const right = rect.width - 10;
    const top = 10;
    const bottom = rect.height - 24;

    let min = Infinity;
    let max = -Infinity;
    let latest = 0;
    for (const signal of this.signals.values()) {
      for (const [time, value] of this.samples(signal)) {
        latest = Math.max(latest, time);
        if (Math.abs(value) < Infinity) {
          min = Math.min(min, value);
          max = Math.max(max, value);
        }
      }
    }
    if (!Number.isFinite(min)) {
      min = -1;
      max = 1;
    }
    if (max - min < 1e-12) {
      const pad = Math.max(1e-6, Math.abs(max) * 0.1);
      min -= pad;
      max += pad;
    }
    const start = Math.max(0, latest - this.windowSeconds);
    const x = (time) => left + ((time - start) / Math.max(1e-12, latest - start)) * (right - left);
    const y = (value) => bottom - ((value - min) / (max - min)) * (bottom - top);

    ctx.strokeStyle = "#243040";
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let i = 0; i <= 4; i += 1) {
      const gy = top + ((bottom - top) * i) / 4;
      ctx.moveTo(left, gy);
      ctx.lineTo(right, gy);
    }
    for (let i = 0; i <= 4; i += 1) {
      const gx = left + ((right - left) * i) / 4;
      ctx.moveTo(gx, top);
      ctx.lineTo(gx, bottom);
    }
    ctx.stroke();

    ctx.fillStyle = "#7f8fa4";
    ctx.font = "10px ui-monospace, monospace";
    ctx.textAlign = "right";
    ctx.fillText(max.toPrecision(3), left - 5, top + 8);
    ctx.fillText(min.toPrecision(3), left - 5, bottom - 2);
    ctx.textAlign = "center";
    ctx.fillText(`${(start * 1000).toFixed(2)} ms`, left + 20, bottom + 15);
    ctx.fillText(`${(latest * 1000).toFixed(2)} ms`, right - 20, bottom + 15);

    let legend = 0;
    let index = 0;
    for (const [id, signal] of this.signals) {
      const color = COLORS[index % COLORS.length];
      ctx.beginPath();
      let started = false;
      for (const [time, value] of this.samples(signal)) {
        if (time < start) continue;
        const px = x(time);
        const py = y(value);
        if (!started) {
          ctx.moveTo(px, py);
          started = true;
        } else {
          ctx.lineTo(px, py);
        }
      }
      ctx.strokeStyle = color;
      ctx.lineWidth = 1.8;
      ctx.stroke();
      ctx.fillStyle = color;
      ctx.textAlign = "left";
      ctx.fillText(`${id} (${signal.unit})`, left + 8 + legend, top + 12);
      legend += 100;
      index += 1;
    }
  }
}
