const assert = require("node:assert/strict");
const test = require("node:test");

const { initializeAutosave } = require("../gui/autosave.mjs");
const panelSource = require("./test-source.cjs").panelSource();

function fakeForm(id) {
  return {
    id, listeners: {}, submits: 0, valid: true,
    addEventListener(type, listener) { this.listeners[type] = listener; },
    checkValidity() { return this.valid; },
    requestSubmit() { this.submits += 1; },
  };
}

function fakeInput(type, tagName = "INPUT") {
  const selectors = {
    'input[type="checkbox"], input[type="radio"], input[type="range"], select':
      tagName === "SELECT" || ["checkbox", "radio", "range"].includes(type),
  };
  return { type, tagName, matches: (selector) => Boolean(selectors[selector]) };
}

function setup() {
  const names = ["botPresenceForm", "routingForm", "systemForm", "mediaForm", "messagesForm", "moderationForm", "commandsForm"];
  const forms = Object.fromEntries(names.map((name) => [name, fakeForm(name)]));
  const states = [];
  const privacyConceptsElement = fakeInput("text");
  const context = { dirtyForms: new Set(Object.values(forms)) };
  initializeAutosave({
    forms: Object.values(forms),
    dirtyForms: context.dirtyForms,
    formSaveState: (form) => form.id,
    setSaveState: (element, state, message) => states.push([element, state, message]),
    t: (key) => key,
    skipTarget: privacyConceptsElement,
    doc: { visibilityState: "visible", addEventListener() {} },
    clock: { setTimeout, clearTimeout },
  });
  return { forms, states, privacyConceptsElement, context };
}

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

test("toggles and menus save shortly after they change", async () => {
  const { forms } = setup();
  forms.mediaForm.listeners.input({ target: fakeInput("checkbox") });
  forms.routingForm.listeners.input({ target: fakeInput("", "SELECT") });
  await wait(350);
  assert.equal(forms.mediaForm.submits, 1);
  assert.equal(forms.routingForm.submits, 1);
});

test("typed values wait until the field is left", async () => {
  const { forms } = setup();
  forms.systemForm.listeners.input({ target: fakeInput("number") });
  await wait(350);
  assert.equal(forms.systemForm.submits, 0);
  forms.systemForm.listeners.change({ target: fakeInput("number") });
  await wait(10);
  assert.equal(forms.systemForm.submits, 1);
});

test("invalid values are reported instead of saved", async () => {
  const { forms, states } = setup();
  forms.messagesForm.valid = false;
  forms.messagesForm.listeners.change({ target: fakeInput("number") });
  await wait(10);
  assert.equal(forms.messagesForm.submits, 0);
  assert.deepEqual(states.at(-1), ["messagesForm", "error", "autosaveInvalid"]);
});

test("filter words keep their own save and clean forms are not resubmitted", async () => {
  const { forms, privacyConceptsElement, context } = setup();
  forms.moderationForm.listeners.change({ target: privacyConceptsElement });
  context.dirtyForms.delete(forms.commandsForm);
  forms.commandsForm.listeners.change({ target: fakeInput("checkbox") });
  await wait(10);
  assert.equal(forms.moderationForm.submits, 0);
  assert.equal(forms.commandsForm.submits, 0);
});

test("the panel wires every settings form and skips the filter words field", () => {
  assert.match(panelSource, /initializeAutosave\(\{\s*forms: \[botPresenceForm, routingForm, systemForm, mediaForm, messagesForm, moderationForm, commandsForm\]/);
  assert.match(panelSource, /skipTarget: privacyConceptsElement/);
});
