// Resolves the stream style shared by every output (notifications, music card, captions).
// Loaded before each output script; also required directly by the Node tests.
(function outputTheme(root) {
  const designs = ["graphite", "paper", "neo-brutalism", "gridline", "lumen", "signal"];
  const styles = [...designs, "subtitle"];

  /** "auto" follows the panel design and theme; any other value is kept if known. */
  function resolveOutputTheme(preferences = {}) {
    const design = designs.includes(preferences.design) ? preferences.design : "graphite";
    const style = styles.includes(preferences.outputStyle) ? preferences.outputStyle : design;
    const background = ["light", "dark"].includes(preferences.outputBackground)
      ? preferences.outputBackground
      : preferences.theme === "light" ? "light" : "dark";
    return { style, background };
  }

  /** Dark or white text for a label drawn on the accent color (WCAG relative luminance). */
  function accentInk(rgb) {
    if (!Array.isArray(rgb) || rgb.length !== 3) return "#07130d";
    const [r, g, b] = rgb.map((channel) => {
      const value = Math.min(255, Math.max(0, Number(channel) || 0)) / 255;
      return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.18 ? "#07130d" : "#ffffff";
  }

  function applyOutputTheme(element, preferences = {}) {
    const { style, background } = resolveOutputTheme(preferences);
    element.dataset.outputStyle = style;
    element.dataset.outputBackground = background;
    element.style.setProperty("--accent-ink", accentInk(preferences.accentRgb));
    return { style, background };
  }

  const api = { designs, styles, resolveOutputTheme, applyOutputTheme, accentInk };
  root.relayOutputTheme = api;
  if (typeof module === "object" && module.exports) module.exports = api;
})(globalThis);
