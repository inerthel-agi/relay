// Custom Discord commands: pure helpers plus the editor on the Commands page.

export function cloneCustomCommands(commands) {
  return JSON.parse(JSON.stringify(Array.isArray(commands) ? commands : []));
}

export function defaultCustomAction(type) {
  const reason = () => ({ mode: "optional", fixedValue: "" });
  const entity = () => ({ mode: "required", fixedValue: "" });
  switch (type) {
    case "ban": return { type, reason: reason(), deleteMessageDays: { mode: "fixed", fixedValue: 0 } };
    case "unban": return { type, reason: reason() };
    case "kick": return { type, reason: reason() };
    case "timeout": return { type, durationMinutes: { mode: "fixed", fixedValue: 60 }, reason: reason() };
    case "removeTimeout": return { type, reason: reason() };
    case "clearMessages": return {
      type,
      channel: { mode: "optional", fixedValue: "" },
      count: { mode: "fixed", fixedValue: 10 },
    };
    case "addRole": return { type, role: entity(), reason: reason() };
    case "removeRole": return { type, role: entity(), reason: reason() };
    case "reply": return { type, text: "Relay", ephemeral: true };
    default: return defaultCustomAction("ban");
  }
}

export function customActionTranslationKey(type) {
  return {
    ban: "customActionBan", unban: "customActionUnban", kick: "customActionKick",
    timeout: "customActionTimeout", removeTimeout: "customActionRemoveTimeout",
    clearMessages: "customActionClearMessages", addRole: "customActionAddRole",
    removeRole: "customActionRemoveRole", reply: "customActionReply",
  }[type] || "customActionReply";
}

export function customActionPermissionKey(type) {
  return {
    ban: "permissionBanMembers", unban: "permissionBanMembers", kick: "permissionKickMembers",
    timeout: "permissionModerateMembers", removeTimeout: "permissionModerateMembers",
    clearMessages: "permissionManageMessages", addRole: "permissionManageRoles",
    removeRole: "permissionManageRoles",
  }[type];
}

export function normalizeDiscordId(value) {
  const match = String(value || "").trim().match(/^(?:\d{17,20}|<@!?(\d{17,20})>|<@&(\d{17,20})>|<#(\d{17,20})>)$/);
  if (!match) return null;
  return match[1] || match[2] || match[3] || match[0];
}

export function discordIdListFromInput(value, invalidMessage = "customInvalidIds") {
  const tokens = String(value || "").split(/[\s,]+/).filter(Boolean);
  const ids = tokens.map(normalizeDiscordId);
  if (ids.some((id) => !id) || ids.length > 100) throw new Error(invalidMessage);
  return [...new Set(ids)];
}

// The editor owns its DOM and draft state; the panel passes the shared helpers it needs.
export function initializeCustomCommands({ $, $$, t, invoke, formatTranslation, setSaveState, getBootstrap, applyConfig, applyBootstrap }) {
  const customCommandForm = $("#custom-command-form");
  const customCommandListElement = $("#custom-command-list");
  const customCommandsEmptyElement = $("#custom-commands-empty");
  const customCommandCountElement = $("#custom-command-count");
  const customCommandsSaveStateElement = $("#custom-commands-save-state");
  const customCommandEditorStateElement = $("#custom-command-editor-state");
  const customCommandPreviewElement = $("#custom-command-preview");
  const customCommandNameElement = $("#custom-command-name");
  const customCommandDescriptionElement = $("#custom-command-description");
  const customCommandActionElement = $("#custom-command-action");
  const customCommandEnabledElement = $("#custom-command-enabled");
  const customActionFieldsElement = $("#custom-action-fields");
  const customCommandAdminOnlyElement = $("#custom-command-admin-only");
  const customCommandUsersElement = $("#custom-command-users");
  const customCommandRolesElement = $("#custom-command-roles");
  const customCommandChannelsElement = $("#custom-command-channels");
  const customRequiredPermissionsElement = $("#custom-required-permissions");
  const customPermissionInputs = $$('input[name="custom-permission"]');
  const addCustomCommandButton = $("#add-custom-command");
  const cancelCustomCommandButton = $("#cancel-custom-command");
  const syncCustomCommandsButton = $("#sync-custom-commands");
  const defaultRelayCommandNames = new Set([
    "channel", "url", "show", "status", "test", "regenerate", "clear", "nuke", "lock", "changelog",
  ]);
  let customCommands = [];
  let customCommandsDirty = false;
  let editingCustomCommandIndex = null;

  function customParameterMarkup(key, labelKey, kind, minimum = "", maximum = "") {
    const inputAttributes = kind === "integer"
      ? `type="number" min="${minimum}" max="${maximum}" step="1"`
      : `type="text" maxlength="512" autocomplete="off" spellcheck="false"`;
    return `
      <div class="custom-parameter" data-custom-parameter="${key}" data-kind="${kind}" data-min="${minimum}" data-max="${maximum}">
        <strong>${t(labelKey)}</strong>
        <label class="field">
          <span>${t("customParameterMode")}</span>
          <select data-custom-parameter-mode>
            <option value="required">${t("parameterRequired")}</option>
            <option value="optional">${t("parameterOptional")}</option>
            <option value="fixed">${t("parameterFixed")}</option>
          </select>
        </label>
        <label class="field">
          <span>${t("customParameterValue")}</span>
          <input data-custom-parameter-value ${inputAttributes}>
        </label>
      </div>`;
  }

  function setCustomParameterValue(key, parameter) {
    const root = customActionFieldsElement.querySelector(`[data-custom-parameter="${key}"]`);
    if (!root) return;
    root.querySelector("[data-custom-parameter-mode]").value = parameter?.mode || "optional";
    root.querySelector("[data-custom-parameter-value]").value = String(parameter?.fixedValue ?? "");
  }

  function updateCustomParameterAvailability(root = customActionFieldsElement) {
    const parameters = root.matches?.("[data-custom-parameter]")
      ? [root]
      : $$('[data-custom-parameter]', root);
    for (const parameter of parameters) {
      const mode = parameter.querySelector("[data-custom-parameter-mode]").value;
      const value = parameter.querySelector("[data-custom-parameter-value]");
      value.disabled = mode === "required";
      value.required = mode === "fixed" || (mode === "optional" && parameter.dataset.kind === "entity-role");
    }
  }

  function renderCustomRequiredPermissions() {
    const permissionKey = customActionPermissionKey(customCommandActionElement.value);
    const permission = permissionKey ? t(permissionKey) : "—";
    customRequiredPermissionsElement.textContent = formatTranslation("customRequiredPermission", { permission });
  }

  function renderCustomActionFields(action = defaultCustomAction(customCommandActionElement.value)) {
    const type = customCommandActionElement.value;
    if (action.type !== type) action = defaultCustomAction(type);
    switch (type) {
      case "ban":
        customActionFieldsElement.innerHTML = customParameterMarkup("reason", "customReason", "text")
          + customParameterMarkup("deleteMessageDays", "customDeleteDays", "integer", 0, 7);
        setCustomParameterValue("reason", action.reason);
        setCustomParameterValue("deleteMessageDays", action.deleteMessageDays);
        break;
      case "unban":
      case "kick":
      case "removeTimeout":
        customActionFieldsElement.innerHTML = customParameterMarkup("reason", "customReason", "text");
        setCustomParameterValue("reason", action.reason);
        break;
      case "timeout":
        customActionFieldsElement.innerHTML = customParameterMarkup("durationMinutes", "customDurationMinutes", "integer", 1, 40320)
          + customParameterMarkup("reason", "customReason", "text");
        setCustomParameterValue("durationMinutes", action.durationMinutes);
        setCustomParameterValue("reason", action.reason);
        break;
      case "clearMessages":
        customActionFieldsElement.innerHTML = customParameterMarkup("channel", "customChannelId", "entity-channel")
          + customParameterMarkup("count", "customMessageCount", "integer", 1, 1000);
        setCustomParameterValue("channel", action.channel);
        setCustomParameterValue("count", action.count);
        break;
      case "addRole":
      case "removeRole":
        customActionFieldsElement.innerHTML = customParameterMarkup("role", "customRoleId", "entity-role")
          + customParameterMarkup("reason", "customReason", "text");
        setCustomParameterValue("role", action.role);
        setCustomParameterValue("reason", action.reason);
        break;
      case "reply":
        customActionFieldsElement.innerHTML = `
          <label class="field field--full">
            <span>${t("customReplyText")}</span>
            <textarea id="custom-reply-text" minlength="1" maxlength="1900" required></textarea>
          </label>
          <label class="field field--full">
            <span>${t("customReplyVisibility")}</span>
            <select id="custom-reply-visibility">
              <option value="ephemeral">${t("customReplyEphemeral")}</option>
              <option value="public">${t("customReplyPublic")}</option>
            </select>
          </label>`;
        $("#custom-reply-text").value = action.text || "";
        $("#custom-reply-visibility").value = action.ephemeral === false ? "public" : "ephemeral";
        break;
    }
    updateCustomParameterAvailability();
    renderCustomRequiredPermissions();
  }

  function readCustomParameter(key) {
    const root = customActionFieldsElement.querySelector(`[data-custom-parameter="${key}"]`);
    const mode = root.querySelector("[data-custom-parameter-mode]").value;
    const input = root.querySelector("[data-custom-parameter-value]");
    let fixedValue = input.value.trim();
    if (root.dataset.kind === "integer") {
      fixedValue = Number(fixedValue || root.dataset.min || 0);
      if (mode !== "required"
        && (!Number.isInteger(fixedValue)
          || fixedValue < Number(root.dataset.min)
          || fixedValue > Number(root.dataset.max))) {
        input.setCustomValidity(t("customParameterValue"));
        input.reportValidity();
        input.setCustomValidity("");
        throw new Error(t("customParameterValue"));
      }
    } else if (root.dataset.kind.startsWith("entity") && fixedValue) {
      const id = normalizeDiscordId(fixedValue);
      if (!id) throw new Error(t("customInvalidIds"));
      fixedValue = id;
    }
    if (root.dataset.kind === "entity-role" && mode !== "required" && !fixedValue) {
      throw new Error(t("customInvalidIds"));
    }
    if (root.dataset.kind === "entity-channel" && mode === "fixed" && !fixedValue) {
      throw new Error(t("customInvalidIds"));
    }
    return { mode, fixedValue };
  }

  function readCustomAction() {
    const type = customCommandActionElement.value;
    switch (type) {
      case "ban": return { type, reason: readCustomParameter("reason"), deleteMessageDays: readCustomParameter("deleteMessageDays") };
      case "unban": return { type, reason: readCustomParameter("reason") };
      case "kick": return { type, reason: readCustomParameter("reason") };
      case "timeout": return { type, durationMinutes: readCustomParameter("durationMinutes"), reason: readCustomParameter("reason") };
      case "removeTimeout": return { type, reason: readCustomParameter("reason") };
      case "clearMessages": return { type, channel: readCustomParameter("channel"), count: readCustomParameter("count") };
      case "addRole": return { type, role: readCustomParameter("role"), reason: readCustomParameter("reason") };
      case "removeRole": return { type, role: readCustomParameter("role"), reason: readCustomParameter("reason") };
      case "reply": return {
        type,
        text: $("#custom-reply-text").value,
        ephemeral: $("#custom-reply-visibility").value !== "public",
      };
      default: throw new Error(t("customCommandAction"));
    }
  }

  function collectCustomCommandDraft() {
    if (!customCommandForm.reportValidity()) return null;
    const name = customCommandNameElement.value.trim().toLowerCase();
    if (defaultRelayCommandNames.has(name)
      || customCommands.some((command, index) => command.name === name && index !== editingCustomCommandIndex)) {
      throw new Error(t("customDuplicateName"));
    }
    return {
      name,
      description: customCommandDescriptionElement.value.trim(),
      enabled: customCommandEnabledElement.checked,
      action: readCustomAction(),
      access: {
        administratorOnly: customCommandAdminOnlyElement.checked,
        requiredPermissions: customPermissionInputs.filter((input) => input.checked).map((input) => input.value),
        allowedUserIds: discordIdListFromInput(customCommandUsersElement.value, t("customInvalidIds")),
        allowedRoleIds: discordIdListFromInput(customCommandRolesElement.value, t("customInvalidIds")),
        allowedChannelIds: discordIdListFromInput(customCommandChannelsElement.value, t("customInvalidIds")),
      },
    };
  }

  function renderCustomCommands() {
    customCommandListElement.replaceChildren();
    customCommandCountElement.textContent = `${customCommands.length} / 16`;
    customCommandsEmptyElement.hidden = customCommands.length !== 0;
    addCustomCommandButton.disabled = customCommands.length >= 16;
    for (const [index, command] of customCommands.entries()) {
      const item = document.createElement("li");
      item.className = "custom-command-card";
      const identity = document.createElement("div");
      identity.className = "custom-command-card__identity";
      const title = document.createElement("span");
      const code = document.createElement("strong");
      code.className = "command-code";
      code.textContent = `/relay ${command.name}`;
      const badge = document.createElement("span");
      badge.className = "custom-command-card__badge";
      badge.dataset.active = String(command.enabled !== false);
      badge.textContent = t(command.enabled !== false ? "active" : "disabled");
      title.append(code, badge);
      const details = document.createElement("small");
      details.textContent = `${t(customActionTranslationKey(command.action?.type))} · ${command.description}`;
      identity.append(title, details);
      const actions = document.createElement("div");
      actions.className = "custom-command-card__actions";
      for (const [action, key] of [["edit", "edit"], ["delete", "delete"]]) {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "button button--quiet";
        button.dataset.customCommandAction = action;
        button.dataset.customCommandIndex = String(index);
        button.textContent = t(key);
        actions.append(button);
      }
      item.append(identity, actions);
      customCommandListElement.append(item);
    }
  }

  function closeCustomCommandEditor() {
    customCommandForm.hidden = true;
    editingCustomCommandIndex = null;
    customCommandEditorStateElement.textContent = "";
    syncCustomCommandsButton.disabled = false;
  }

  function openCustomCommandEditor(index = null) {
    if (index === null && customCommands.length >= 16) {
      setSaveState(customCommandsSaveStateElement, "error", t("customMaxReached"));
      return;
    }
    editingCustomCommandIndex = index;
    const definition = index === null ? {
      name: "",
      description: "",
      enabled: true,
      action: defaultCustomAction("ban"),
      access: { administratorOnly: true, requiredPermissions: [], allowedUserIds: [], allowedRoleIds: [], allowedChannelIds: [] },
    } : cloneCustomCommands([customCommands[index]])[0];
    customCommandNameElement.value = definition.name;
    customCommandDescriptionElement.value = definition.description;
    customCommandEnabledElement.checked = definition.enabled !== false;
    customCommandActionElement.value = definition.action.type;
    customCommandAdminOnlyElement.checked = definition.access?.administratorOnly !== false;
    const requiredPermissions = new Set(definition.access?.requiredPermissions || []);
    for (const input of customPermissionInputs) input.checked = requiredPermissions.has(input.value);
    customCommandUsersElement.value = (definition.access?.allowedUserIds || []).join("\n");
    customCommandRolesElement.value = (definition.access?.allowedRoleIds || []).join("\n");
    customCommandChannelsElement.value = (definition.access?.allowedChannelIds || []).join("\n");
    customCommandPreviewElement.textContent = `/relay ${definition.name || "command"}`;
    renderCustomActionFields(definition.action);
    customCommandForm.hidden = false;
    syncCustomCommandsButton.disabled = true;
    customCommandNameElement.focus();
  }

  addCustomCommandButton.addEventListener("click", () => openCustomCommandEditor());
  cancelCustomCommandButton.addEventListener("click", closeCustomCommandEditor);

  customCommandNameElement.addEventListener("input", () => {
    const normalized = customCommandNameElement.value
      .toLowerCase()
      .replace(/\s+/g, "-")
      .replace(/[^a-z0-9_-]/g, "");
    if (normalized !== customCommandNameElement.value) customCommandNameElement.value = normalized;
    customCommandPreviewElement.textContent = `/relay ${normalized || "command"}`;
    customCommandEditorStateElement.textContent = "";
  });

  customCommandActionElement.addEventListener("change", () => {
    renderCustomActionFields(defaultCustomAction(customCommandActionElement.value));
    customCommandEditorStateElement.textContent = "";
  });

  customActionFieldsElement.addEventListener("change", (event) => {
    if (event.target.matches("[data-custom-parameter-mode]")) {
      updateCustomParameterAvailability(event.target.closest("[data-custom-parameter]"));
    }
    customCommandEditorStateElement.textContent = "";
  });

  customCommandForm.addEventListener("submit", (event) => {
    event.preventDefault();
    customCommandEditorStateElement.textContent = "";
    try {
      const definition = collectCustomCommandDraft();
      if (!definition) return;
      if (editingCustomCommandIndex === null) customCommands.push(definition);
      else customCommands[editingCustomCommandIndex] = definition;
      customCommandsDirty = true;
      renderCustomCommands();
      closeCustomCommandEditor();
      setSaveState(customCommandsSaveStateElement, "unsaved", t("customDraftSaved"));
    } catch (error) {
      customCommandEditorStateElement.textContent = String(error.message || error);
    }
  });

  customCommandListElement.addEventListener("click", (event) => {
    const button = event.target.closest("[data-custom-command-action]");
    if (!button) return;
    const index = Number(button.dataset.customCommandIndex);
    if (!Number.isInteger(index) || !customCommands[index]) return;
    if (button.dataset.customCommandAction === "edit") {
      openCustomCommandEditor(index);
      return;
    }
    customCommands.splice(index, 1);
    customCommandsDirty = true;
    closeCustomCommandEditor();
    renderCustomCommands();
    setSaveState(customCommandsSaveStateElement, "unsaved", t("customUnsaved"));
  });

  syncCustomCommandsButton.addEventListener("click", async () => {
    setSaveState(customCommandsSaveStateElement, "saving", t("customValidating"));
    const names = new Set();
    const invalidName = customCommands.some((command) => {
      if (defaultRelayCommandNames.has(command.name) || names.has(command.name)) return true;
      names.add(command.name);
      return false;
    });
    if (customCommands.length > 16 || invalidName) {
      setSaveState(customCommandsSaveStateElement, "error", t("customDuplicateName"));
      return;
    }
    syncCustomCommandsButton.disabled = true;
    addCustomCommandButton.disabled = true;
    setSaveState(customCommandsSaveStateElement, "saving", t("customSyncing"));
    try {
      const config = await invoke("save_custom_commands", { commands: cloneCustomCommands(customCommands) });
      customCommandsDirty = false;
      customCommands = cloneCustomCommands(config.customCommands);
      getBootstrap().config = config;
      applyConfig(config);
      try {
        applyBootstrap(await invoke("get_bootstrap"));
      } catch {
        renderCustomCommands();
      }
      setSaveState(customCommandsSaveStateElement, "saved", t("customActive"));
    } catch (error) {
      customCommandsDirty = true;
      setSaveState(customCommandsSaveStateElement, "error", String(error));
    } finally {
      syncCustomCommandsButton.disabled = false;
      addCustomCommandButton.disabled = customCommands.length >= 16;
    }
  });

  return {
    render: renderCustomCommands,
    // Re-translate the list and, if open, the editor fields without losing the draft.
    applyLanguage() {
      renderCustomCommands();
      if (customCommandForm.hidden) return;
      let action;
      try {
        action = readCustomAction();
      } catch {
        action = defaultCustomAction(customCommandActionElement.value);
      }
      renderCustomActionFields(action);
    },
    // Config reloads replace the list unless the user has unsynchronized edits.
    syncFromConfig(config) {
      if (customCommandsDirty) return;
      customCommands = cloneCustomCommands(config.customCommands);
      renderCustomCommands();
    },
  };
}
