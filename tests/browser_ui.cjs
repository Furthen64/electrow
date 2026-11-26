#!/usr/bin/env node
/*
 * Browser UI integration harness for the Electrow dashboard and schematic editor.
 *
 * Requires a running local backend and a Chrome instance started with
 * --remote-debugging-port. tests/run_browser_ui.sh starts both automatically.
 *
 * Usage:
 *   node tests/browser_ui.cjs [--cdp http://127.0.0.1:9222] [--base http://127.0.0.1:8000] [--out Temp/browser-ui]
 *
 * Node 22+ is required for the global WebSocket client.
 */

const fs = require("node:fs");
const path = require("node:path");

function parseArgs(argv) {
  const options = {
    cdp: "http://127.0.0.1:9222",
    base: "http://127.0.0.1:8000",
    out: "Temp/browser-ui",
  };
  for (let index = 2; index < argv.length; index += 2) {
    const key = argv[index]?.replace(/^--/, "");
    const value = argv[index + 1];
    if (!key || value === undefined || !(key in options)) {
      throw new Error(`invalid argument at ${index}: ${argv[index]}`);
    }
    options[key] = value;
  }
  return options;
}

const wait = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

function report(name, ok, detail = "") {
  const label = ok ? "PASS" : "FAIL";
  console.log(`${label}  ${name}${detail ? ` — ${detail}` : ""}`);
  return ok;
}

async function main() {
  const options = parseArgs(process.argv);
  const port = new URL(options.base).port;
  fs.mkdirSync(options.out, { recursive: true });

  const targets = await (await fetch(`${options.cdp}/json/list`)).json();
  const page = targets.find(
    (target) => target.type === "page" && target.url.includes(port)
  );
  if (!page) throw new Error(`no Chrome page found for port ${port}`);

  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });

  let nextId = 0;
  const pending = new Map();
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      pending.get(message.id)(message);
      pending.delete(message.id);
    }
  };
  const send = (method, params = {}) =>
    new Promise((resolve) => {
      const id = ++nextId;
      pending.set(id, resolve);
      ws.send(JSON.stringify({ id, method, params }));
    });
  const evaluate = async (expression) => {
    const reply = await send("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (reply.result?.exceptionDetails) {
      throw new Error(reply.result.exceptionDetails.text);
    }
    return reply.result?.result?.value;
  };
  const click = async (id) => {
    await evaluate(`document.getElementById('${id}').click()`);
    await wait(350);
  };
  const text = (id) => evaluate(`document.getElementById('${id}').textContent`);
  const exportCount = async () => {
    await click("btn-export");
    const raw = await text("export-output");
    return JSON.parse(raw).elements.length;
  };
  const screenshot = async (name) => {
    const reply = await send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(options.out, name), Buffer.from(reply.result.data, "base64"));
  };

  const results = [];
  await send("Runtime.enable");
  await send("Page.enable");
  await send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1100,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await send("Page.reload", { ignoreCache: true });
  await wait(1200);

  const palette = await evaluate("document.getElementById('palette').children.length");
  results.push(report("palette renders components", palette === 9, `${palette} buttons`));

  await click("btn-divider");
  await wait(700);
  const divider = await evaluate(`({
    revision: Number(document.getElementById('stat-revision').textContent),
    time: document.getElementById('stat-time').textContent,
    diagnostics: document.getElementById('diagnostics-list').textContent,
    measurements: document.getElementById('measurements-body').innerText,
    nodes: document.getElementById('nodes-body').innerText
  })`);
  results.push(report("divider demo builds topology", divider.revision >= 5, `revision ${divider.revision}`));
  results.push(report("divider demo has a healthy solve", divider.diagnostics.includes("None"), divider.diagnostics));
  results.push(report("divider midpoint is 5 V", divider.measurements.includes("5.00000 V") && divider.nodes.includes("5.00000 V")));

  await evaluate("document.getElementById('schematic').scrollIntoView({block:'center'})");
  await wait(250);
  await screenshot("schematic.png");

  await click("btn-run");
  await wait(1000);
  await click("btn-pause");
  const afterRun = await evaluate(`({
    time: document.getElementById('stat-time').textContent,
    scopePixels: (() => {
      const canvas = document.getElementById('scope');
      const ctx = canvas.getContext('2d');
      const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      let lit = 0;
      for (let i = 0; i < data.length; i += 40) {
        if (data[i] > 32 || data[i + 1] > 32 || data[i + 2] > 48) lit += 1;
      }
      return lit;
    })()
  })`);
  const parsedTime = Number.parseFloat(afterRun.time);
  results.push(report("run advances simulated time", parsedTime > 0.0001, afterRun.time));
  results.push(report("scope draws traces", afterRun.scopePixels > 10, `${afterRun.scopePixels} lit samples`));
  await evaluate("document.getElementById('scope').scrollIntoView({block:'center'})");
  await wait(200);
  await screenshot("scope.png");

  const before = await exportCount();
  await evaluate(`[...document.querySelectorAll('#palette button')].find(b => b.textContent === 'capacitor').click()`);
  await wait(450);
  const added = await exportCount();
  await click("btn-undo");
  const undone = await exportCount();
  await click("btn-redo");
  const redone = await exportCount();
  results.push(report("palette placement adds an element", added === before + 1, `${before} -> ${added}`));
  results.push(report("undo removes the placed element", undone === before, `${added} -> ${undone}`));
  results.push(report("redo restores the placed element", redone === added, `${undone} -> ${redone}`));

  const name = "browser-ui-check";
  await evaluate(`document.getElementById('file-name').value = '${name}'`);
  await click("btn-save-file");
  const saveResponse = await text("response");
  await click("btn-new");
  const cleared = await exportCount();
  await click("btn-load-file");
  const restored = await exportCount();
  results.push(report("save file succeeds", saveResponse.includes(name), saveResponse.slice(0, 80)));
  results.push(report("new circuit clears the editor", cleared === 0, `${cleared} elements`));
  results.push(report("load file restores elements", restored === redone, `${restored} elements`));

  const dump =
    "$ 1 5.0E-6 10 63 10.0 62\n" +
    "v 112 368 112 48 0 0 40.0 10.0 0.0\n" +
    "r 112 48 240 48 0 1000\n" +
    "w 240 48 112 368 0\n";
  await evaluate(`document.getElementById('dump-input').value = ${JSON.stringify(dump)}`);
  await click("btn-load-dump");
  const importResponse = await text("response");
  const imported = await exportCount();
  results.push(report("Falstad dump imports", imported === 3 && !importResponse.includes("invalid"), `${imported} elements`));

  ws.close();
  const failed = results.filter((result) => !result).length;
  console.log(`\n${results.length - failed}/${results.length} browser checks passed. Screenshots in ${options.out}/`);
  if (failed) process.exit(1);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
