const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const { resolveOutputTheme, applyOutputTheme, accentInk, styles } = require("../outputs/theme.js");

const themeCss = fs.readFileSync(__dirname + "/../outputs/theme.css", "utf8");

test("automatic output style follows the Relay design and theme", () => {
  assert.deepEqual(resolveOutputTheme({ design: "paper", theme: "light" }), { style: "paper", background: "light" });
  assert.deepEqual(resolveOutputTheme({ design: "signal", theme: "dark", outputStyle: "auto", outputBackground: "auto" }), { style: "signal", background: "dark" });
  assert.deepEqual(resolveOutputTheme({ design: "paper", theme: "light", outputStyle: "subtitle", outputBackground: "dark" }), { style: "subtitle", background: "dark" });
  // Unknown or missing values fall back to Graphite, dark: the look before this setting existed.
  assert.deepEqual(resolveOutputTheme({ design: "openai", outputStyle: "psn" }), { style: "graphite", background: "dark" });
  assert.deepEqual(resolveOutputTheme(), { style: "graphite", background: "dark" });
});

test("the applied style lands on the root element with a readable accent label color", () => {
  const properties = {};
  const element = { dataset: {}, style: { setProperty: (name, value) => { properties[name] = value; } } };
  applyOutputTheme(element, { design: "lumen", theme: "light", accentRgb: [255, 220, 90] });
  assert.equal(element.dataset.outputStyle, "lumen");
  assert.equal(element.dataset.outputBackground, "light");
  assert.equal(properties["--accent-ink"], "#07130d");
  assert.equal(accentInk([40, 30, 160]), "#ffffff");
  assert.equal(accentInk(undefined), "#07130d");
});

test("every output style is defined in the shared stylesheet", () => {
  for (const style of styles.filter((name) => name !== "graphite")) {
    assert.match(themeCss, new RegExp(`:root\\[data-output-style="${style}"\\] \\{`), style);
    assert.match(themeCss, new RegExp(`:root\\[data-output-style="${style}"\\]\\[data-output-background="light"\\]`), style);
  }
});

// Resolves the tokens the cascade applies for one style and background.
function tokens(style, background) {
  const blocks = [...themeCss.matchAll(/(:root[^{]*)\{([^}]*)\}/g)].map(([, selector, body]) => ({
    style: selector.match(/data-output-style="([^"]+)"/)?.[1] || null,
    background: selector.match(/data-output-background="([^"]+)"/)?.[1] || null,
    values: Object.fromEntries([...body.matchAll(/(--out-[\w-]+):\s*([^;]+);/g)].map(([, name, value]) => [name, value.trim()])),
  }));
  const pick = (blockStyle, blockBackground) => blocks.find((block) => block.style === blockStyle && block.background === blockBackground)?.values || {};
  return {
    ...pick(null, null),
    ...(background === "light" ? pick(null, "light") : {}),
    ...(style === "graphite" ? {} : pick(style, null)),
    ...(background === "light" && style !== "graphite" ? pick(style, "light") : {}),
  };
}

function luminance(hex) {
  const value = hex.replace("#", "");
  return [0, 2, 4].map((index) => Number.parseInt(value.slice(index, index + 2), 16) / 255)
    .map((channel) => (channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4))
    .reduce((sum, channel, index) => sum + channel * [0.2126, 0.7152, 0.0722][index], 0);
}

function contrast(first, second) {
  const [light, dark] = [luminance(first), luminance(second)].sort((a, b) => b - a);
  return (light + 0.05) / (dark + 0.05);
}

test("message and name text keep at least 4.5:1 contrast on every card", () => {
  for (const style of styles.filter((name) => name !== "subtitle")) {
    for (const background of ["light", "dark"]) {
      const values = tokens(style, background);
      for (const token of ["--out-ink", "--out-muted"]) {
        const ratio = contrast(values[token], values["--out-surface"]);
        assert.ok(ratio >= 4.5, `${style} ${background} ${token}: ${ratio.toFixed(2)}`);
      }
    }
  }
});
