// Personalization → Notification style: how notifications, the music card and
// media captions look on stream. "auto" follows the Relay design and theme.

export const streamStyles = ["auto", "graphite", "paper", "neo-brutalism", "gridline", "lumen", "signal", "subtitle"];
export const streamBackgrounds = ["auto", "light", "dark"];
const designNames = {
  graphite: "Graphite", paper: "Paper", "neo-brutalism": "Neo-Brutalism",
  gridline: "Gridline", lumen: "Lumen", signal: "Signal",
};

export function normalizeStreamStyle(value) {
  return streamStyles.includes(value) ? value : "auto";
}

export function normalizeStreamBackground(value) {
  return streamBackgrounds.includes(value) ? value : "auto";
}

/** Status line under the controls, e.g. "Follows Relay: Paper, Dark." */
export function describeStreamStyle({ style, background, design, theme }, t, formatTranslation) {
  const isAutomatic = style === "auto" && background === "auto";
  const resolvedStyle = style === "auto" ? design : style;
  const resolvedBackground = background === "auto" ? theme : background;
  const styleName = resolvedStyle === "subtitle" ? t("streamStyleSubtitle") : designNames[resolvedStyle] || designNames.graphite;
  return {
    isAutomatic,
    text: formatTranslation(isAutomatic ? "streamStyleFollows" : "streamStyleCustom", {
      style: styleName,
      background: t(resolvedBackground === "light" ? "light" : "dark"),
    }),
  };
}

export function initializeStreamStyle({ $, $$, t, formatTranslation, readStorage, getDesign, getTheme, onChange }) {
  const styleInputs = $$("input[name='stream-style']");
  const backgroundInputs = $$("input[name='stream-background']");
  const status = $("#stream-style-status");
  const resetButton = $("#stream-style-reset");
  const preview = $("#stream-style-preview");
  let style = normalizeStreamStyle(readStorage("relay-output-style"));
  let background = normalizeStreamBackground(readStorage("relay-output-background"));

  function store() {
    try {
      localStorage.setItem("relay-output-style", style);
      localStorage.setItem("relay-output-background", background);
    } catch { /* Storage is optional; Relay keeps the saved preference. */ }
  }

  function render() {
    for (const input of styleInputs) input.checked = input.value === style;
    for (const input of backgroundInputs) input.checked = input.value === background;
    const summary = describeStreamStyle({ style, background, design: getDesign(), theme: getTheme() }, t, formatTranslation);
    status.textContent = summary.text;
    resetButton.hidden = summary.isAutomatic;
  }

  function update(next) {
    style = normalizeStreamStyle(next.style ?? style);
    background = normalizeStreamBackground(next.background ?? background);
    store();
    render();
    onChange();
  }

  for (const input of styleInputs) input.addEventListener("change", () => update({ style: input.value }));
  for (const input of backgroundInputs) input.addEventListener("change", () => update({ background: input.value }));
  resetButton.addEventListener("click", () => {
    update({ style: "auto", background: "auto" });
    styleInputs[0]?.focus();
  });

  /** The notification page in preview mode receives appearance changes live. */
  function setPreviewPort(port, language) {
    if (!port) return;
    const url = new URL(`http://127.0.0.1:${port}/notifications`);
    url.searchParams.set("preview", "1");
    url.searchParams.set("lang", language);
    if (preview.src !== url.href) preview.src = url.href;
  }

  function reset() {
    style = "auto";
    background = "auto";
    store();
    render();
  }

  render();
  return {
    render,
    reset,
    setPreviewPort,
    applyLanguage: render,
    preferences: () => ({ outputStyle: style, outputBackground: background }),
  };
}
