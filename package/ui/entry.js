const POLL_INTERVAL_MS = 10_000;
const KIRO_ICON_SVG = "__KIRO_ICON_SVG__";

function formatNumber(value) {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 }).format(value);
}

function formatRefreshTime(value) {
  if (typeof value !== "number") return "Never";
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(new Date(value));
}

function displayMessage(state) {
  if (state.refreshing) return "Refreshing usage…";
  if (state.error) return state.error;
  return "Kiro usage is unavailable.";
}

function usageMarkup(usage) {
  if (!usage) return "";
  const note = usage.note ? `<p class="note">${escapeHtml(usage.note)}</p>` : "";
  return `
    <dl>
      <div><dt>Plan</dt><dd>${escapeHtml(usage.plan)}</dd></div>
      <div><dt>Credits covered</dt><dd>${formatNumber(usage.covered_credits)} / ${formatNumber(usage.total_credits)}</dd></div>
      <div><dt>Coverage</dt><dd>${formatNumber(usage.covered_percent)}%</dd></div>
      <div><dt>Resets</dt><dd>${escapeHtml(usage.reset_date)}</dd></div>
      <div><dt>Last refreshed</dt><dd>${formatRefreshTime(usage.fetched_at_ms)}</dd></div>
    </dl>
    ${note}`;
}

function escapeHtml(value) {
  return String(value).replace(/[&<>'"]/g, (character) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    "'": "&#39;",
    '"': "&quot;",
  })[character]);
}

function usageTone(usage) {
  if (!usage) return "muted";
  if (usage.covered_percent >= 95) return "danger";
  if (usage.covered_percent >= 80) return "warning";
  return "normal";
}

function mountTitlebar(root, context) {
  root.innerHTML = `
    <style>
      :host { display: block; height: 100%; }
      button { display: inline-flex; align-items: center; justify-content: center; gap: 4px; width: 100%; height: 100%; min-height: 0; padding: 0 4px; border: 0; border-radius: 4px; background: transparent; color: var(--planeai-text-muted); font-size: 11px; white-space: nowrap; }
      .kiro-icon { width: 18px; height: 18px; flex: none; }
      .usage { font-variant-numeric: tabular-nums; }
      button:hover, button:focus-visible { background: var(--planeai-accent-subtle); color: var(--planeai-text); outline: none; }
      button.warning .usage { color: var(--planeai-warning); }
      button.danger .usage { color: var(--planeai-danger); }
      button.muted .usage { color: var(--planeai-text-subtle); }
    </style>
    <button type="button" aria-label="Open Kiro usage details">${KIRO_ICON_SVG}<span class="usage" data-usage>—</span></button>`;
  const button = root.querySelector("button");
  const usageLabel = root.querySelector("[data-usage]");
  let disposed = false;

  const render = async () => {
    try {
      const state = await context.host.call("usage.status");
      if (disposed) return;
      const usage = state.usage;
      usageLabel.textContent = usage ? `${formatNumber(usage.covered_percent)}%` : "—";
      button.className = state.stale ? "muted" : usageTone(usage);
      button.title = state.stale ? `Stale: ${displayMessage(state)}` : usage ? `${formatNumber(usage.covered_percent)}% covered by plan` : displayMessage(state);
    } catch (error) {
      if (!disposed) {
        usageLabel.textContent = "—";
        button.className = "muted";
        button.title = String(error);
      }
    }
  };

  const openDetails = () => context.host.navigation.open("kiro-usage", "details");
  button.addEventListener("click", openDetails);
  void render();
  const poller = setInterval(() => void render(), POLL_INTERVAL_MS);
  return () => {
    disposed = true;
    clearInterval(poller);
    button.removeEventListener("click", openDetails);
    root.replaceChildren();
  };
}

function mountDetails(root, context) {
  root.innerHTML = `
    <style>
      :host { display: block; height: 100%; }
      main { min-height: 100%; padding: 16px; background: var(--planeai-main); color: var(--planeai-text); }
      section { padding: 20px; border: 1px solid var(--planeai-border); border-radius: 8px; background: var(--planeai-surface); }
      h1 { margin: 0 0 8px; }
      p { margin: 0; color: var(--planeai-text-muted); }
      p.error { color: var(--planeai-danger); }
      p.stale { color: var(--planeai-warning); }
      dl { display: grid; gap: 12px; margin: 20px 0; }
      dl div { display: grid; grid-template-columns: 150px 1fr; gap: 12px; }
      dt { color: var(--planeai-text-muted); }
      dd { margin: 0; font-variant-numeric: tabular-nums; }
      .note { margin-top: 16px; font-size: 12px; }
      button { margin-top: 20px; }
    </style>
    <main><section><h1>Kiro Usage</h1><p data-status role="status">Loading…</p><div data-usage></div><button type="button" data-refresh>Refresh now</button></section></main>`;
  const status = root.querySelector("[data-status]");
  const usageElement = root.querySelector("[data-usage]");
  const refreshButton = root.querySelector("[data-refresh]");
  let disposed = false;

  const render = async () => {
    try {
      const state = await context.host.call("usage.status");
      if (disposed) return;
      const usage = state.usage;
      usageElement.innerHTML = usageMarkup(usage);
      status.className = state.stale ? "stale" : state.error ? "error" : "";
      if (state.stale) status.textContent = `Showing stale usage. ${displayMessage(state)}`;
      else if (usage) status.textContent = state.refreshing ? "Refreshing usage…" : "Credits are covered by your Kiro plan.";
      else status.textContent = displayMessage(state);
      refreshButton.disabled = Boolean(state.refreshing);
      refreshButton.textContent = state.refreshing ? "Refreshing…" : "Refresh now";
    } catch (error) {
      if (!disposed) {
        status.className = "error";
        status.textContent = String(error);
      }
    }
  };

  const refresh = async () => {
    refreshButton.disabled = true;
    try {
      await context.host.call("usage.refresh");
    } catch (error) {
      if (!disposed) {
        status.className = "error";
        status.textContent = String(error);
      }
    }
    await render();
  };

  refreshButton.addEventListener("click", refresh);
  void render();
  const poller = setInterval(() => void render(), POLL_INTERVAL_MS);
  return () => {
    disposed = true;
    clearInterval(poller);
    refreshButton.removeEventListener("click", refresh);
    root.replaceChildren();
  };
}

export default {
  mount(root, context) {
    return context.contribution.id === "titlebar"
      ? mountTitlebar(root, context)
      : mountDetails(root, context);
  },
};
