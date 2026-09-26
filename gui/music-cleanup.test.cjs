const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const panelHtml = fs.readFileSync(__dirname + "/panel.html", "utf8");
const panelSource = require("./test-source.cjs").panelSource();
const dictionarySource = panelSource.slice(
  panelSource.indexOf("const translations ="),
  panelSource.indexOf("const pageMetadata ="),
);
const dictionaryContext = vm.createContext({});
vm.runInContext(`${dictionarySource}\nglobalThis.translationsForTest = translations;`, dictionaryContext);
const translations = dictionaryContext.translationsForTest;

function setup() {
  const elements = new Map();
  const element = (id) => {
    if (!elements.has(id)) elements.set(id, { hidden: true, disabled: false, textContent: "",
      handlers: {}, addEventListener(name, callback) { this.handlers[name] = callback; } });
    return elements.get(id);
  };
  const calls = [];
  const routingForm = element("form");
  const dirtyForms = new Set();
  const source = fs.readFileSync(__dirname + "/panel.js", "utf8");
  const block = source.slice(source.indexOf("let musicCleanupToken;"), source.indexOf('refreshChannelsButton.addEventListener("click"'));
  const context = vm.createContext({ $: element, routingForm, dirtyForms,
    t: (key) => key,
    invoke: async (command, args) => {
      calls.push({ command, args });
      return command === "preview_music_cleanup" ? { token: "snapshot-7", count: 3 } : { deleted: 3, failed: 0, skipped: 0 };
    },
  });
  vm.runInContext(block, context);
  return { element, calls, routingForm, dirtyForms };
}

test("preview requires a separate confirmation and sends only its snapshot token", async () => {
  const ui = setup();
  await ui.element("#music-cleanup-preview").handlers.click();
  assert.equal(ui.calls.length, 1);
  assert.equal(ui.element("#music-cleanup-confirmation").hidden, false);
  await ui.element("#music-cleanup-confirm").handlers.click();
  assert.equal(ui.calls[1].command, "confirm_music_cleanup");
  assert.equal(ui.calls[1].args.token, "snapshot-7");
  await ui.element("#music-cleanup-confirm").handlers.click();
  assert.equal(ui.calls.length, 2);
});

test("cancel and settings edits invalidate the cleanup confirmation", async () => {
  for (const cancel of [true, false]) {
    const ui = setup();
    await ui.element("#music-cleanup-preview").handlers.click();
    if (cancel) ui.element("#music-cleanup-cancel").handlers.click();
    else ui.routingForm.handlers.input();
    await ui.element("#music-cleanup-confirm").handlers.click();
    assert.equal(ui.calls.length, 1);
  }
});

test("unsaved channel settings prevent cleanup previews", async () => {
  const ui = setup();
  ui.dirtyForms.add(ui.routingForm);
  await ui.element("#music-cleanup-preview").handlers.click();
  assert.equal(ui.calls.length, 0);
});

test("music welcome protection is optional in every supported language", () => {
  assert.match(panelHtml, /data-i18n="musicWelcome">Protected welcome message \(optional\)<\/span>/);
  assert.match(panelHtml, /data-i18n="musicWelcomeHelp">Leave empty to protect no welcome message\./);
  assert.match(panelHtml, /data-i18n="musicCleanupHelp">Requires Manage Messages\. A protected welcome message is optional/);

  const expected = {
    en: { optional: "(optional)", empty: "Leave empty to protect no welcome message.", never: "never deleted", cleanup: "welcome message is optional" },
    fr: { optional: "(facultatif)", empty: "Laissez vide pour ne protéger aucun message d’accueil.", never: "jamais supprimé", cleanup: "message d’accueil protégé est facultatif" },
    es: { optional: "(opcional)", empty: "Déjalo vacío para no proteger ninguna bienvenida.", never: "nunca se elimina", cleanup: "bienvenida protegida es opcional" },
    de: { optional: "(optional)", empty: "Leer lassen, wenn keine Willkommensnachricht geschützt werden soll.", never: "nie gelöscht", cleanup: "Willkommensnachricht ist optional" },
    ru: { optional: "(необязательно)", empty: "Оставьте поле пустым, чтобы не защищать приветственное сообщение.", never: "никогда не удаляется", cleanup: "приветственное сообщение необязательно" },
    zh: { optional: "（可选）", empty: "留空表示不保护任何欢迎消息。", never: "永不删除", cleanup: "欢迎消息为可选项" },
    ko: { optional: "(선택 사항)", empty: "보호할 환영 메시지가 없으면 비워 두세요.", never: "삭제하지 않습니다", cleanup: "환영 메시지는 선택 사항" },
    ja: { optional: "（任意）", empty: "保護する案内メッセージがない場合は空欄にします。", never: "削除されません", cleanup: "案内メッセージは任意" },
    id: { optional: "(opsional)", empty: "Kosongkan jika tidak ada pesan sambutan yang ingin dilindungi.", never: "tidak pernah dihapus", cleanup: "bersifat opsional" },
  };

  for (const [language, markers] of Object.entries(expected)) {
    assert.match(translations[language].musicWelcome, new RegExp(`${markers.optional.replace(/[()（）]/g, "\\$&")}$`), `${language}.musicWelcome`);
    assert.match(translations[language].musicWelcomeHelp, new RegExp(markers.empty), `${language}.musicWelcomeHelp empty`);
    assert.match(translations[language].musicWelcomeHelp, new RegExp(markers.never), `${language}.musicWelcomeHelp never`);
    assert.match(translations[language].musicCleanupHelp, new RegExp(markers.cleanup), `${language}.musicCleanupHelp optional`);
  }
});
