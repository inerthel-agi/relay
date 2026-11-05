const assert = require("node:assert/strict");
const test = require("node:test");
const vm = require("node:vm");

const source = require("./test-source.cjs").panelSource();
const apply = source.slice(source.indexOf("function applyNotificationSoundConfig"), source.indexOf("async function refreshRuntimeStatus"));

function render(config) {
  const element = () => ({ checked: false, hidden: false, value: "", textContent: "" });
  const context = {
    t: (key) => key,
    notificationSoundEnabledElement: element(),
    notificationSoundObsElement: element(),
    notificationSoundChoiceElement: element(),
    pickNotificationSoundButton: element(),
    clearNotificationSoundButton: element(),
    notificationSoundStateElement: element(),
  };
  vm.createContext(context);
  vm.runInContext(apply, context);
  context.applyNotificationSoundConfig(config);
  return context;
}

test("Relay's own sounds hide the file buttons", () => {
  const view = render({ notificationSoundChoice: "chat", notificationSoundPath: "C:/relay/ping.mp3" });
  assert.equal(view.notificationSoundChoiceElement.value, "chat");
  assert.equal(view.pickNotificationSoundButton.hidden, true);
  assert.equal(view.clearNotificationSoundButton.hidden, true);
  assert.equal(view.notificationSoundStateElement.textContent, "");
  assert.equal(render({}).notificationSoundChoiceElement.value, "soft");
});

test("a file chosen before sound choices stays selected as the custom sound", () => {
  const view = render({ notificationSoundPath: "C:\\Users\\me\\ping.mp3" });
  assert.equal(view.notificationSoundChoiceElement.value, "custom");
  assert.equal(view.pickNotificationSoundButton.hidden, false);
  assert.equal(view.clearNotificationSoundButton.hidden, false);
  assert.equal(view.notificationSoundStateElement.textContent, "ping.mp3");
});

test("a custom choice without a file asks for one", () => {
  const view = render({ notificationSoundChoice: "custom" });
  assert.equal(view.pickNotificationSoundButton.hidden, false);
  assert.equal(view.clearNotificationSoundButton.hidden, true);
  assert.equal(view.notificationSoundStateElement.textContent, "noNotificationSound");
});
