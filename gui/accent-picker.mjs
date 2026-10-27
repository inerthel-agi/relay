// Accent color: preset swatches, a HEX field and a themed picker that replaces the
// grey system popup of <input type="color">. #accent-picker stays the value holder:
// every change goes through its "input" event, which panel.js already applies and saves.

/** "#RRGGBB" in lower case, or null for anything else ("#" is optional). */
export function normalizeHex(value) {
  const hex = String(value).trim().replace(/^#?/, "#").toLowerCase();
  return /^#[0-9a-f]{6}$/.test(hex) ? hex : null;
}

export function hexToRgbArray(hex) {
  return [1, 3, 5].map((index) => parseInt(hex.slice(index, index + 2), 16));
}

export function rgbArrayToHex(rgb) {
  return `#${rgb.map((value) => Math.round(Math.min(255, Math.max(0, value))).toString(16).padStart(2, "0")).join("")}`;
}

/** Hue in degrees, saturation and value from 0 to 1. */
export function rgbToHsv([red, green, blue]) {
  const [r, g, b] = [red / 255, green / 255, blue / 255];
  const max = Math.max(r, g, b);
  const delta = max - Math.min(r, g, b);
  let hue = 0;
  if (delta) {
    if (max === r) hue = ((g - b) / delta) % 6;
    else if (max === g) hue = (b - r) / delta + 2;
    else hue = (r - g) / delta + 4;
  }
  return [(hue * 60 + 360) % 360, max ? delta / max : 0, max];
}

export function hsvToRgb([hue, saturation, value]) {
  const channel = (n) => {
    const k = (n + hue / 60) % 6;
    return (value - value * saturation * Math.max(0, Math.min(k, 4 - k, 1))) * 255;
  };
  return [channel(5), channel(3), channel(1)];
}

/** True when dark text reads better than white on this color (same threshold as outputs). */
export function prefersDarkText(hex) {
  const [r, g, b] = hexToRgbArray(hex).map((channel) => {
    const value = channel / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.18;
}

export function initializeAccentPicker({ t, doc = document, win = window }) {
  const picker = doc.querySelector("#accent-picker");
  const swatches = [...doc.querySelectorAll("[data-accent-hex]")];
  const custom = doc.querySelector("#accent-custom");
  const popover = doc.querySelector("#accent-popover");
  const area = doc.querySelector("#accent-area");
  const thumb = area.querySelector(".accent-popover__thumb");
  const hue = doc.querySelector("#accent-hue");
  const popoverHex = doc.querySelector("#accent-popover-hex");
  const dot = popover.querySelector(".accent-popover__dot");
  const dropper = doc.querySelector("#accent-dropper");
  const hexInput = doc.querySelector("#accent-hex");
  const chip = doc.querySelector(".accent-hex__chip");
  const contrast = doc.querySelector("#accent-contrast");
  let hsv = [0, 0, 0];

  for (const swatch of swatches) swatch.style.setProperty("--swatch", swatch.dataset.accentHex);

  function setAccent(hex) {
    picker.value = hex;
    picker.dispatchEvent(new Event("input", { bubbles: true }));
    render();
  }

  /** Mirrors the current accent; panel.js may also change it (reset, other windows). */
  function render() {
    const hex = picker.value.toLowerCase();
    if (doc.activeElement !== hexInput) hexInput.value = hex.toUpperCase();
    chip.style.background = hex;
    custom.style.setProperty("--swatch", hex);
    contrast.textContent = t(prefersDarkText(hex) ? "accentTextDark" : "accentTextLight");
    let preset = false;
    for (const swatch of swatches) {
      const on = swatch.dataset.accentHex === hex;
      preset ||= on;
      swatch.setAttribute("aria-checked", String(on));
    }
    custom.toggleAttribute("data-active", !preset);
  }

  function drawPopover() {
    area.style.setProperty("--hue", `hsl(${hsv[0]} 100% 50%)`);
    thumb.style.left = `${hsv[1] * 100}%`;
    thumb.style.top = `${(1 - hsv[2]) * 100}%`;
    area.setAttribute("aria-valuetext", `${Math.round(hsv[1] * 100)}% · ${Math.round(hsv[2] * 100)}%`);
    hue.value = String(Math.round(hsv[0]));
    const hex = rgbArrayToHex(hsvToRgb(hsv));
    dot.style.background = hex;
    thumb.style.background = hex;
    if (doc.activeElement !== popoverHex) popoverHex.value = hex.toUpperCase();
  }
  function commit() {
    drawPopover();
    setAccent(rgbArrayToHex(hsvToRgb(hsv)));
  }
  function openPopover(open) {
    popover.hidden = !open;
    custom.setAttribute("aria-expanded", String(open));
    if (open) {
      hsv = rgbToHsv(hexToRgbArray(picker.value));
      drawPopover();
      area.focus();
    }
  }

  for (const swatch of swatches) swatch.addEventListener("click", () => setAccent(swatch.dataset.accentHex));
  custom.addEventListener("click", () => openPopover(popover.hidden));
  doc.addEventListener("pointerdown", (event) => {
    if (!popover.hidden && !custom.parentElement.contains(event.target)) openPopover(false);
  });
  popover.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    openPopover(false);
    custom.focus();
  });
  const pointAt = (event) => {
    const box = area.getBoundingClientRect();
    hsv[1] = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
    hsv[2] = 1 - Math.min(1, Math.max(0, (event.clientY - box.top) / box.height));
    commit();
  };
  area.addEventListener("pointerdown", (event) => {
    area.setPointerCapture(event.pointerId);
    pointAt(event);
  });
  area.addEventListener("pointermove", (event) => {
    if (area.hasPointerCapture(event.pointerId)) pointAt(event);
  });
  area.addEventListener("keydown", (event) => {
    const step = event.shiftKey ? 0.1 : 0.02;
    const move = { ArrowLeft: [1, -step], ArrowRight: [1, step], ArrowUp: [2, step], ArrowDown: [2, -step] }[event.key];
    if (!move) return;
    event.preventDefault();
    hsv[move[0]] = Math.min(1, Math.max(0, hsv[move[0]] + move[1]));
    commit();
  });
  hue.addEventListener("input", () => {
    hsv[0] = Number(hue.value);
    commit();
  });
  for (const input of [hexInput, popoverHex]) {
    input.addEventListener("change", () => {
      const hex = normalizeHex(input.value);
      if (!hex) {
        render();
        drawPopover();
        return;
      }
      hsv = rgbToHsv(hexToRgbArray(hex));
      drawPopover();
      setAccent(hex);
    });
  }
  // The screen color picker exists in WebView2 (Chromium); hidden elsewhere.
  dropper.hidden = !("EyeDropper" in win);
  dropper.addEventListener("click", async () => {
    try {
      const { sRGBHex } = await new win.EyeDropper().open();
      hsv = rgbToHsv(hexToRgbArray(sRGBHex));
      commit();
    } catch { /* Cancelled. */ }
  });
  picker.addEventListener("input", render);
  render();
  return { render };
}
