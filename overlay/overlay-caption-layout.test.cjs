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

test("shared text scaling does not add padding to audio metadata", () => {
  const shared = styles.slice(styles.indexOf(".widget-move-badge,"), styles.indexOf("\n}", styles.indexOf(".widget-move-badge,")));
  assert.doesNotMatch(shared, /padding:|max-height:|transform-origin:/);
  assert.match(shared, /font-size: calc\(1em \* var\(--font-scale\)\)/);
});

test("audio cards anchor to the right edge without a horizontal translation", () => {
  const card = styles.slice(styles.indexOf(".audio-card {"), styles.indexOf(".audio-card[hidden]"));
  assert.match(card, /right: 4px/);
  assert.match(card, /transform-origin: right top/);
  assert.doesNotMatch(card, /translate\(-50%/);
});
