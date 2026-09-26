export function roleChoicesForChannels(catalog, channelIds) {
  const guilds = new Set(catalog.channels.filter(channel => channelIds.includes(channel.id)).map(channel => channel.guildId));
  return guilds.size ? catalog.roles.filter(role => guilds.has(role.guildId)) : catalog.roles;
}

export function renderAccessMenu(root, { choices, selectedIds, kind, onChange, t }) {
  root.replaceChildren();
  const heading = document.createElement("span");
  heading.textContent = t(kind === "channel" ? "modAllowedChannels" : "modAllowedRoles");
  root.append(heading);
  const rows = document.createElement("div"); rows.className = "reaction-access__rows"; root.append(rows);
  const selects = [];
  const emit = () => onChange([...new Set(selects.map(select => select.value).filter(Boolean))]);
  const placeholder = t(kind === "channel" ? "modChooseChannel" : "modChooseRole");
  function addRow(id = "") {
    const row = document.createElement("div"); row.className = "reaction-access__row";
    const select = document.createElement("select"); select.setAttribute("aria-label", heading.textContent);
    const empty = document.createElement("option"); empty.value = ""; empty.textContent = placeholder; select.append(empty);
    const groups = new Map();
    for (const choice of choices) {
      if (!groups.has(choice.guildId)) {
        const group = document.createElement("optgroup"); group.label = choice.guildName; groups.set(choice.guildId, group); select.append(group);
      }
      const option = document.createElement("option"); option.value = choice.id;
      option.textContent = `${kind === "channel" ? "# " : ""}${choice.name} — ${choice.guildName}`;
      groups.get(choice.guildId).append(option);
    }
    if (id && !choices.some(choice => choice.id === id)) {
      const unavailable = document.createElement("option"); unavailable.value = id;
      unavailable.textContent = `${t("modUnknownChoice")} — ${id}`; select.append(unavailable);
    }
    select.value = id;
    select.addEventListener("change", emit); selects.push(select);
    const remove = document.createElement("button"); remove.type = "button"; remove.className = "button button--quiet";
    remove.textContent = t("modRemove");
    remove.addEventListener("click", () => { selects.splice(selects.indexOf(select), 1); row.remove(); emit(); });
    row.append(select, remove); rows.append(row);
    return select;
  }
  for (const id of selectedIds.length ? selectedIds : [""]) addRow(id);
  const add = document.createElement("button"); add.type = "button"; add.className = "button button--quiet";
  add.textContent = t(kind === "channel" ? "modAddChannel" : "modAddRole");
  add.addEventListener("click", () => { if (selects.length < 100) addRow().focus(); });
  root.append(add);
}
