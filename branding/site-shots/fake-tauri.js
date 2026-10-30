// Website screenshots only: fake Tauri backend with demo data. Nothing reaches Discord, OBS or the real config.
(() => {
  const params = new URLSearchParams(location.search);
  localStorage.setItem("relay-theme", params.get("theme") || "dark");
  localStorage.setItem("relay-design", params.get("design") || "graphite");
  localStorage.setItem("relay-interface-mode", params.get("mode") || "simple");
  localStorage.setItem("relay-language", "en");
  localStorage.setItem("relay-locale", "en-US");

  const now = Date.now();
  const geometry = () => ({ cropTop: 0, cropRight: 0, cropBottom: 0, cropLeft: 0, contentScale: 100, anchor: "legacy", marginX: 12, marginY: 16 });
  const output = (obs, widget = 0) => ({ obsClients: obs, widgetClients: widget, previewClients: 0, lastConnectedAt: now - 4 * 60000 });
  const author = (username) => ({ id: username, username, avatarUrl: null });
  const media = (id, kind, file, who, ago) => ({
    id, messageId: String(id), kind, filename: file, url: `./demo/${file}`, contentType: kind === "gif" ? "image/gif" : "image/jpeg",
    author: author(who), timestamp: now - ago * 1000,
  });
  const config = {
    musicMaxPendingPerUser: 3, musicRejectDuplicatePending: true,
    reactions: { enabled: true, items: [], sounds: [], visuals: [] },
    watchedChannelId: "1001", formerMessageChannelId: "", mediaCleanupEnabled: false, mediaWelcomeMessageId: "",
    formerMessageCleanupEnabled: false, formerMessageWelcomeMessageId: "",
    musicChannelId: "1002", musicCleanupEnabled: false, musicWelcomeMessageId: "",
    honeypotChannelId: "1003", honeypotAction: "kick", port: 4590,
    displayDurationMs: 15000, gifDurationMs: 10000, stickerDurationMs: 8000, notificationDurationMs: 8000,
    mediaVolume: 50, skipShortcut: "Ctrl+Alt+S", panicShortcut: "Ctrl+Alt+P",
    notificationCharacterLimit: 0, notificationQueueLimit: 50, notificationsObsEnabled: true,
    botOnlineStatus: "online", botActivityType: "custom", botActivityText: "Watching #clips",
    showAuthor: true, showMediaTextObs: false, showMediaTextWidget: false,
    moderationEnabled: true, moderationAllowImages: true, moderationAllowVideos: true, moderationAllowAudio: true,
    privacyScanEnabled: true, privacySimilarityBoost: 4, privacyConcepts: [], privacyFilterExemptRoleIds: [],
    privacyProtectionLevel: "balanced", privacyEnabledCategories: [], privacyBlockThreshold: "high",
    privacyReviewIntermediate: true, privacyAutoDeleteBlockedMessages: true, privacyAllowlist: [], privacyCustomPatterns: [],
    moderation: {
      wordPacks: [], packExclusions: [], filterScopes: { messages: true, media: true, music: true },
      filterUsernames: false, filterMusicTitles: false, userCooldownSeconds: 20, raidLimitPerMinute: 30,
      blockDuplicates: true, blockTextSpam: true, minAccountAgeDays: 7, minMemberAgeDays: 0,
      trustedRoleIds: [], trustedUserIds: [], blockedUserIds: [], blockInvites: true, blockShorteners: true,
      blockScamDomains: true, reviewOnlySelected: false, reviewText: false, pendingExpiryMinutes: 30,
      safetyDelaySeconds: 0, warnOnBlock: false, escalationBlocks: 0, escalationWindowMinutes: 10,
      escalationTimeoutMinutes: 10, honeypotExemptRoleIds: [], honeypotTimeoutMinutes: 10, livePreset: null,
      liveObsPort: 4455, liveRestore: null, maxMediaSeconds: 0, loudnessLimiter: true, cooldownNotice: true,
      duplicateWindowSeconds: 60,
    },
    ai: { provider: "openai", modelTier: "fast", maxCallsPerHour: 120, maxCallsPerDay: 1000, imageGuard: true, personalDataRelay: true, messageAdvice: true, watchMessages: true },
    commandChannelEnabled: true, commandUrlEnabled: true, commandShowEnabled: true, commandStatusEnabled: true,
    commandTestEnabled: true, commandRegenerateEnabled: true, commandClearEnabled: true, commandNukeEnabled: true,
    commandLockEnabled: true, commandChangelogEnabled: true, customCommands: [], channelLock: null,
    interfacePreferences: { language: "en", theme: "dark", accentRgb: [88, 185, 137], fontScale: 100, design: "graphite", outputStyle: "auto", outputBackground: "auto" },
    widgetX: null, widgetY: null, widgetWidth: 480, widgetHeight: 270, widgetKeepAspectRatio: true, widgetVisible: false, widgetLocked: false, widgetSoundEnabled: false,
    notificationWidgetX: null, notificationWidgetY: null, notificationWidgetWidth: 420, notificationWidgetHeight: 160,
    notificationWidgetVisible: true, notificationWidgetLocked: false,
    mediaObsGeometry: geometry(), mediaWidgetGeometry: geometry(), notificationObsGeometry: geometry(),
    notificationWidgetGeometry: geometry(), musicObsGeometry: geometry(), musicVideoObsGeometry: geometry(),
    notificationSoundEnabled: false, notificationSoundObsEnabled: false, notificationSoundPath: null,
  };
  const pendingMedia = [
    { id: 41, media: media(41, "image", "ace.jpg", "Pixel", 25), privacyClassification: "low", privacyReason: "manual_review",
      aiAdvice: { state: "done", verdict: "noIssueSeen", categories: [], reason: "A game screenshot with no text about a person." } },
    { id: 42, media: media(42, "gif", "gg.jpg", "Nova", 12), privacyClassification: "low", privacyReason: "manual_review" },
  ];
  const queue = [
    { id: 51, ready: true, title: "clutch.jpg", author: "Nova", kind: "image" },
    { id: 52, ready: true, title: "Victory dance", author: "Kiwi", kind: "gif" },
    { id: 53, ready: false, title: "", author: "Orbit", kind: "video" },
  ];
  // ?fresh=1: no media relayed yet, so the setup checklist still has its last step.
  const fresh = params.get("fresh") === "1";
  if (fresh) localStorage.removeItem("relay-setup-tested");
  const history = fresh ? [] : [
    media(31, "image", "meme.jpg", "Nova", 60), media(32, "gif", "gg.jpg", "Kiwi", 180),
    media(33, "image", "ace.jpg", "Pixel", 420), media(34, "image", "nice.jpg", "Orbit", 900),
  ];
  const status = () => ({
    bot: { connected: true, username: "Relay", displayAvatarUrl: null, error: null },
    server: { connected: true, overlayClients: 4, outputs: { reaction: output(1), visual: output(1), audio: output(1), notification: output(1, 1), sticker: output(1) }, error: null },
    channels: [
      { id: "1001", name: "clips", guildName: "Night Owls" },
      { id: "1002", name: "music-requests", guildName: "Night Owls" },
      { id: "1003", name: "do-not-post", guildName: "Night Owls" },
    ],
    widget: { visible: false, locked: false },
    notificationWidget: { visible: true, locked: false },
    pendingMedia, queue, outputsPaused: false,
  });
  const responses = {
    get_bootstrap: () => ({
      ...status(), config,
      credentials: { configured: true, source: "keyring", clientId: "123", youtubeConfigured: true, obsPasswordConfigured: true, openaiConfigured: true, anthropicConfigured: false },
      history, overlayUrl: "http://127.0.0.1:4590/overlay", audioUrl: "http://127.0.0.1:4590/audio",
      wsUrl: "ws://127.0.0.1:1/ws?token=preview", inviteUrl: "https://discord.com/oauth2/authorize?client_id=123",
    }),
    get_runtime_status: status,
    apply_config: (args) => { Object.assign(config, args?.config || {}); return responses.get_bootstrap(); },
    get_app_version: () => "1.5.0",
    get_changelog_markdown: () => "## [1.5.0]\n\nNew logo, Control room, Simple and Advanced modes, AI image check.",
    get_message_status: () => ({
      pinned: true,
      current: { author: author("Mod Ava"), text: "Tournament starts at 9 pm. Post your best clips in #clips!" },
    }),
    get_media_library: () => ({ items: [], totalBytes: 0 }),
    get_music_queue: () => ({ current: null, queue: [] }),
    get_reactions: () => ({ items: [] }),
    get_reaction_access_options: () => ({ roles: [], users: [] }),
    get_start_with_windows: () => true,
    check_discord_setup: () => null,
    moderation_overview: () => ({
      packs: [], activePreset: "standard", liveStreaming: false, raidActive: false,
      summary: { blocked: 3, held: 5, ignored: 12, flaggedMembers: 1 },
      authorNames: { u1: "Spammy", u2: "Kiwi", u3: "Orbit" },
      entries: [
        { at: now - 40000, action: "held", reason: "manual_review", lane: "media", authorId: "u2" },
        { at: now - 90000, action: "ignored", reason: "cooldown", lane: "notification", authorId: "u3" },
        { at: now - 150000, action: "blocked", reason: "discord_invite", lane: "notification", authorId: "u1" },
        { at: now - 300000, action: "approved", reason: "manual_review", lane: "media", authorId: "u2" },
      ],
      pendingTexts: [
        { id: 61, text: "free skins here, just log in with your account!!", author: author("Spammy"), reason: "ai_flagged", timestamp: now - 8000,
          aiAdvice: { state: "done", verdict: "clearProblem", categories: ["spam"], reason: "Pushes viewers to log in on an outside site." } },
      ],
    }),
  };
  window.__TAURI__ = {
    core: {
      invoke: async (command, args) => {
        const reply = responses[command];
        return reply ? structuredClone(reply(args)) : (args?.config ?? null);
      },
    },
    event: { listen: async () => () => {} },
    window: { getCurrentWindow: () => ({ setTitle() {}, onCloseRequested() {} }) },
  };

  const still = document.createElement("link");
  still.rel = "stylesheet";
  still.href = "./shots.css";
  document.head.append(still);

  // ?page=live opens that page once the panel is ready.
  const page = params.get("page");
  if (page) {
    window.addEventListener("load", () => setTimeout(() => {
      document.querySelector(`[data-page-target="${page}"]`)?.click();
      for (const id of (params.get("open") || "").split(",").filter(Boolean)) document.getElementById(id)?.setAttribute("open", "");
      for (const id of (params.get("close") || "").split(",").filter(Boolean)) document.getElementById(id)?.removeAttribute("open");
      const target = params.get("scroll");
      // Only the page column scrolls, so the top bar and sidebar stay in the shot.
      setTimeout(() => {
        const main = document.querySelector("main.pages");
        const element = target && document.querySelector(target);
        if (main && element) main.scrollTop += element.getBoundingClientRect().top - main.getBoundingClientRect().top - 24;
      }, 400);
    }, 600));
  }
})();
