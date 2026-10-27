// Help page: symptom tabs and the pace of "Ask the AI".

export const AI_HELP_COOLDOWN_MS = 5 * 60 * 1000;
export const AI_HELP_DAILY_LIMIT = 10;
const DAY_MS = 24 * 60 * 60 * 1000;

/** Questions of the last 24 hours, the wait before the next one and how many remain today. */
export function aiHelpQuota(asks, now) {
  const recent = (Array.isArray(asks) ? asks : [])
    .filter((at) => Number.isFinite(at) && at <= now && now - at < DAY_MS);
  const last = recent.length ? Math.max(...recent) : -Infinity;
  const wait = Math.max(0, AI_HELP_COOLDOWN_MS - (now - last));
  const left = Math.max(0, AI_HELP_DAILY_LIMIT - recent.length);
  return { recent, wait, left };
}

/** "4:05" for a wait in milliseconds, rounded up to the second. */
export function formatWait(ms) {
  const seconds = Math.ceil(ms / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

export function initializeHelpUi({ formatTranslation, loadAsks, doc = document, clock = window }) {
  const tabs = [...doc.querySelectorAll("[data-help-symptom]")];
  function select(tab, focus = false) {
    for (const other of tabs) {
      const active = other === tab;
      other.setAttribute("aria-selected", String(active));
      other.tabIndex = active ? 0 : -1;
      doc.getElementById(other.getAttribute("aria-controls")).hidden = !active;
    }
    if (focus) tab.focus();
  }
  for (const [index, tab] of tabs.entries()) {
    tab.addEventListener("click", () => select(tab));
    // Arrow keys move between symptoms, as in any tab list.
    tab.addEventListener("keydown", (event) => {
      const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
      if (!step) return;
      event.preventDefault();
      select(tabs[(index + step + tabs.length) % tabs.length], true);
    });
  }
  if (tabs.length) select(tabs.find((tab) => tab.getAttribute("aria-selected") === "true") || tabs[0]);

  const askButton = doc.querySelector("#ai-help-ask");
  const quotaElement = doc.querySelector("#ai-help-quota");
  // Relay applies the pace; the panel only shows it from Relay's own count.
  let asks = [];
  function render() {
    const { wait, left } = aiHelpQuota(asks, Date.now());
    askButton.disabled = wait > 0 || left === 0;
    quotaElement.textContent = left === 0
      ? formatTranslation("aiHelpQuotaDone", { total: AI_HELP_DAILY_LIMIT })
      : wait > 0
        ? formatTranslation("aiHelpQuotaWait", { time: formatWait(wait), left })
        : formatTranslation("aiHelpQuotaLeft", { left, total: AI_HELP_DAILY_LIMIT });
  }
  async function refresh() {
    try { asks = await loadAsks(); } catch { /* Keep the last count. */ }
    render();
  }
  clock.setInterval(render, 1000);
  render();
  refresh();
  return {
    canAsk: () => {
      const { wait, left } = aiHelpQuota(asks, Date.now());
      return wait === 0 && left > 0;
    },
    refresh,
    render,
  };
}
