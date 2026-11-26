const COLORS = {
  wire: "#93a4bb",
  resistor: "#e7c56a",
  voltage_source: "#7ad7ff",
  current_source: "#ff9ecb",
  capacitor: "#a8e6a1",
  inductor: "#c3a6ff",
  diode: "#ffb07a",
  switch: "#d6dee9",
  ground: "#9fb0c3",
};

function endpoints(element) {
  return [element.x, element.y, element.x2, element.y2];
}

function angle(element) {
  return Math.atan2(element.y2 - element.y, element.x2 - element.x);
}

function line(ctx, x1, y1, x2, y2, width = 2, color = "#8fa2b8") {
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2, y2);
  ctx.lineWidth = width;
  ctx.strokeStyle = color;
  ctx.stroke();
}

function circle(ctx, x, y, radius, color) {
  ctx.beginPath();
  ctx.arc(x, y, radius, 0, Math.PI * 2);
  ctx.lineWidth = 2;
  ctx.strokeStyle = color;
  ctx.stroke();
}

function label(ctx, text, x, y, color = "#c7d3e2") {
  ctx.fillStyle = color;
  ctx.font = "12px ui-monospace, monospace";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(text, x, y);
}

const symbols = {
  wire(ctx, element) {
    const [x1, y1, x2, y2] = endpoints(element);
    line(ctx, x1, y1, x2, y2, 2, COLORS.wire);
  },
  resistor(ctx, element) {
    const [x1, y1, x2, y2] = endpoints(element);
    const dx = x2 - x1;
    const dy = y2 - y1;
    const length = Math.hypot(dx, dy) || 1;
    const ux = dx / length;
    const uy = dy / length;
    const px = -uy;
    const py = ux;
    const a = 0.25;
    const b = 0.75;
    const half = 9;
    line(ctx, x1, y1, x1 + dx * a, y1 + dy * a, 2, COLORS.resistor);
    ctx.beginPath();
    ctx.moveTo(x1 + dx * a + px * half, y1 + dy * a + py * half);
    ctx.lineTo(x1 + dx * b + px * half, y1 + dy * b + py * half);
    ctx.lineTo(x1 + dx * b - px * half, y1 + dy * b - py * half);
    ctx.lineTo(x1 + dx * a - px * half, y1 + dy * a - py * half);
    ctx.closePath();
    ctx.lineWidth = 2;
    ctx.strokeStyle = COLORS.resistor;
    ctx.stroke();
    line(ctx, x1 + dx * b, y1 + dy * b, x2, y2, 2, COLORS.resistor);
    const value = element.params.resistance_ohm;
    label(ctx, `${formatValue(value)}Ω`, (x1 + x2) / 2 + px * 22, (y1 + y2) / 2 + py * 22, COLORS.resistor);
  },
  voltage_source(ctx, element) {
    const [x1, y1, x2, y2] = endpoints(element);
    const cx = (x1 + x2) / 2;
    const cy = (y1 + y2) / 2;
    line(ctx, x1, y1, cx, cy, 2, COLORS.voltage_source);
    circle(ctx, cx, cy, 16, COLORS.voltage_source);
    const a = angle(element);
    const ux = Math.cos(a - Math.PI / 2);
    const uy = Math.sin(a - Math.PI / 2);
    label(ctx, "+", cx + ux * 7, cy + uy * 7, COLORS.voltage_source);
    label(ctx, "−", cx - ux * 7, cy - uy * 7, COLORS.voltage_source);
    line(ctx, cx, cy, x2, y2, 2, COLORS.voltage_source);
    label(ctx, `${formatValue(element.params.voltage_v)}V`, cx + 28, cy - 18, COLORS.voltage_source);
  },
  current_source(ctx, element) {
    const [x1, y1, x2, y2] = endpoints(element);
    const cx = (x1 + x2) / 2;
    const cy = (y1 + y2) / 2;
    line(ctx, x1, y1, cx, cy, 2, COLORS.current_source);
    circle(ctx, cx, cy, 16, COLORS.current_source);
    const a = angle(element);
    ctx.save();
    ctx.translate(cx, cy);
    ctx.rotate(a);
    line(ctx, -8, 0, 8, 0, 2, COLORS.current_source);
    line(ctx, 3, -5, 8, 0, 2, COLORS.current_source);
    line(ctx, 3, 5, 8, 0, 2, COLORS.current_source);
    ctx.restore();
    line(ctx, cx, cy, x2, y2, 2, COLORS.current_source);
    label(ctx, `${formatValue(element.params.current_a)}A`, cx + 28, cy - 18, COLORS.current_source);
  },
  capacitor(ctx, element) {
    const [x1, y1, x2, y2] = endpoints(element);
    const cx = (x1 + x2) / 2;
    const cy = (y1 + y2) / 2;
    const a = angle(element);
    const ux = Math.cos(a + Math.PI / 2);
    const uy = Math.sin(a + Math.PI / 2);
    line(ctx, x1, y1, cx - ux * 8, cy - uy * 8, 2, COLORS.capacitor);
    line(ctx, cx - ux * 8, cy - uy * 8, cx + ux * 8, cy + uy * 8, 3, COLORS.capacitor);
    line(ctx, cx - ux * 8 + (x2 - x1) * 0.06, cy - uy * 8 + (y2 - y1) * 0.06,
      cx + ux * 8 + (x2 - x1) * 0.06, cy + uy * 8 + (y2 - y1) * 0.06, 3, COLORS.capacitor);
    line(ctx, cx + ux * 8 + (x2 - x1) * 0.06, cy + uy * 8 + (y2 - y1) * 0.06, x2, y2, 2, COLORS.capacitor);
    label(ctx, `${formatValue(element.params.capacitance_f)}F`, cx + 24, cy - 18, COLORS.capacitor);
  },
  inductor(ctx, element) {
    const [x1, y1, x2, y2] = endpoints(element);
    const cx = (x1 + x2) / 2;
    const cy = (y1 + y2) / 2;
    const a = angle(element);
    ctx.save();
    ctx.translate(cx, cy);
    ctx.rotate(a);
    ctx.beginPath();
    for (let i = 0; i < 4; i += 1) ctx.arc(-18 + i * 12, 0, 6, Math.PI, 0);
    ctx.lineWidth = 2;
    ctx.strokeStyle = COLORS.inductor;
    ctx.stroke();
    ctx.restore();
    line(ctx, x1, y1, cx - 24 * Math.cos(a), cy - 24 * Math.sin(a), 2, COLORS.inductor);
    line(ctx, cx + 24 * Math.cos(a), cy + 24 * Math.sin(a), x2, y2, 2, COLORS.inductor);
    label(ctx, `${formatValue(element.params.inductance_h)}H`, cx, cy - 20, COLORS.inductor);
  },
  diode(ctx, element) {
    const [x1, y1, x2, y2] = endpoints(element);
    const cx = (x1 + x2) / 2;
    const cy = (y1 + y2) / 2;
    const a = angle(element);
    ctx.save();
    ctx.translate(cx, cy);
    ctx.rotate(a);
    ctx.beginPath();
    ctx.moveTo(-10, -10);
    ctx.lineTo(8, 0);
    ctx.lineTo(-10, 10);
    ctx.closePath();
    ctx.lineWidth = 2;
    ctx.strokeStyle = COLORS.diode;
    ctx.stroke();
    line(ctx, 8, -12, 8, 12, 3, COLORS.diode);
    ctx.restore();
    line(ctx, x1, y1, cx - 10 * Math.cos(a), cy - 10 * Math.sin(a), 2, COLORS.diode);
    line(ctx, cx + 10 * Math.cos(a), cy + 10 * Math.sin(a), x2, y2, 2, COLORS.diode);
    label(ctx, "▶|", cx, cy - 20, COLORS.diode);
  },
  switch(ctx, element) {
    const [x1, y1, x2, y2] = endpoints(element);
    const closed = element.params.closed === 1;
    line(ctx, x1, y1, (x1 + x2) / 2 - 8, y1, 2, COLORS.switch);
    line(ctx, (x1 + x2) / 2 - 8, y1, (x1 + x2) / 2 + 8, closed ? y1 : y1 - 14, 2, COLORS.switch);
    line(ctx, (x1 + x2) / 2 + 8, y1, x2, y2, 2, COLORS.switch);
    label(ctx, closed ? "closed" : "open", (x1 + x2) / 2, y1 - 22, COLORS.switch);
  },
  ground(ctx, element) {
    const x = element.x;
    const y = element.y;
    line(ctx, x, y, x, y + 8, 2, COLORS.ground);
    for (let i = 0; i < 3; i += 1) {
      const width = 16 - i * 5;
      line(ctx, x - width, y + 8 + i * 5, x + width, y + 8 + i * 5, 2, COLORS.ground);
    }
  },
};

export function formatValue(value) {
  if (value === undefined) return "?";
  const absolute = Math.abs(value);
  if (absolute >= 1e6) return `${(value / 1e6).toPrecision(3)}M`;
  if (absolute >= 1e3) return `${(value / 1e3).toPrecision(3)}k`;
  if (absolute > 0 && absolute < 1e-3) return `${(value * 1e6).toPrecision(3)}µ`;
  if (absolute > 0 && absolute < 1) return `${(value * 1e3).toPrecision(3)}m`;
  return String(Number(value).toPrecision(3));
}

export function drawSymbol(ctx, element, style = {}) {
  const draw = symbols[element.kind];
  if (!draw) return;
  ctx.save();
  if (style.selected) {
    ctx.shadowColor = "#4f9dff";
    ctx.shadowBlur = 10;
  }
  draw(ctx, element, style);
  ctx.restore();
}

export function hitTest(element, x, y) {
  if (element.kind === "ground") {
    return Math.abs(x - element.x) <= 18 && y >= element.y - 4 && y <= element.y + 26;
  }
  const dx = element.x2 - element.x;
  const dy = element.y2 - element.y;
  const lengthSquared = dx * dx + dy * dy || 1;
  const t = Math.max(0, Math.min(1, ((x - element.x) * dx + (y - element.y) * dy) / lengthSquared));
  const px = element.x + t * dx;
  const py = element.y + t * dy;
  return Math.hypot(x - px, y - py) <= 12;
}

export { COLORS };
