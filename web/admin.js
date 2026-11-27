const output = document.getElementById("admin-log");
const status = document.getElementById("admin-status");
const autoRefresh = document.getElementById("auto-refresh");

let previous = "";

function setBadge(text, kind) {
  status.textContent = text;
  status.className = `badge badge-${kind}`;
}

async function refresh() {
  try {
    const response = await fetch("/api/logs");
    const data = await response.json();
    const entries = data.entries || [];
    const text = entries
      .map((entry) => `[${entry.ts}] ${entry.level.toUpperCase().padEnd(5)} ${entry.message}`)
      .join("\n") || "No entries yet.";
    if (text !== previous) {
      output.textContent = text;
      output.scrollTop = output.scrollHeight;
      previous = text;
    }
    setBadge(`connected · ${entries.length} entries`, "good");
  } catch (error) {
    setBadge("disconnected", "bad");
  }
}

document.getElementById("btn-refresh").addEventListener("click", refresh);
setInterval(() => {
  if (autoRefresh.checked) refresh();
}, 1000);
refresh();
