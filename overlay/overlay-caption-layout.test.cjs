const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const styles = require("./test-source.cjs").styles();

test("compact widget captions preserve their content width", () => {
  assert.match(styles, /\.overlay__caption\s*\{[\s\S]*?width:\s*fit-content;/);

  const mobileCaption = styles.match(
    /@media \(max-width: 640px\)\s*\{\s*\.overlay__caption\s*\{([\s\S]*?)\n\s*\}/,
  );
  assert.ok(mobileCaption);
  assert.doesNotMatch(mobileCaption[1], /\bright\s*:/);
  assert.match(mobileCaption[1], /max-width:\s*min\(34ch, calc\(100% - 2rem\)\)/);
  assert.match(styles, /\.audio-card__caption\s*\{[\s\S]*?font-size:\s*calc\(14px \* var\(--font-scale\)\)/);
});

test("every text of the media and audio cards follows the text size setting", () => {
  // Rules in cascade order (overlay.css, then audio-card.css). For each text,
  // the last font-size declared for its selector is the one that applies.
  const rules = [...styles.replace(/\/\*[\s\S]*?\*\//g, "").matchAll(/([^{}]+)\{([^{}]*)\}/g)]
    .map(([, selectors, body]) => ({ selectors: selectors.split(",").map((selector) => selector.trim()), body }));
  const appliedFontSize = (selector) => rules
    .filter((rule) => rule.selectors.includes(selector))
    .flatMap((rule) => [...rule.body.matchAll(/(?:^|;)\s*font(?:-size)?:\s*([^;]+)/g)].map((match) => match[1]))
    .at(-1);
  for (const selector of [
    ".overlay__author", ".overlay__caption",
    ".audio-card__label", ".audio-card__title", ".audio-card__artist",
    ".audio-card__caption", ".audio-card__author", ".audio-card__time",
    ".youtube-credit__title", ".youtube-credit__added",
    ".youtube-credit--widget .youtube-credit__label", ".youtube-credit--widget .youtube-credit__title",
    ".youtube-credit--widget .youtube-credit__source", ".youtube-credit--widget .youtube-credit__meta",
  ]) {
    assert.match(appliedFontSize(selector) || "", /var\(--font-scale\)/, selector);
  }
  // Three caption lines stay visible at any text size.
  assert.match(styles, /\.overlay__caption\s*\{[^}]*max-height:\s*calc\(4\.2em \+ 20px\);/);
});

test("audio cards anchor to the right edge without a horizontal translation", () => {
  const card = styles.slice(styles.indexOf(".audio-card {"), styles.indexOf(".audio-card[hidden]"));
  assert.match(card, /right: 4px/);
  assert.match(card, /transform-origin: right top/);
  assert.doesNotMatch(card, /translate\(-50%/);
});
