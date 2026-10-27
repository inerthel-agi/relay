# Changelog

All notable user-facing changes to Relay are documented in this file.

This changelog is maintained in English. Interface translations are maintained in the application.

## Versioning policy

- A major Relay update increments the middle number: `1.0.0` → `1.1.0`.
- A minor update, bug fix, or simple addition increments the patch number: `1.0.0` → `1.0.1`.
- The updater also accepts lettered corrective tags with a lowercase suffix after the patch number, such as `v1.3.6g`.
- Changes remain under `Unreleased` until the matching GitHub release is published.

## [Unreleased]

## [1.5.0] - 2026-10-27

### English

#### Added

- **AI image check.** Moderation → AI assistant → Block harmful images checks each image of a member before it appears. OpenAI's free moderation filter looks for sexual content, violence and self-harm (with your OpenAI key, even when Anthropic is your provider); your provider's paid model looks for hate and extremism, harassment, and spam, with one request per image. An image the AI finds or suspects harmful is blocked, and its Discord message is deleted unless you turn that off; the moderation log names the category that matched (for example Hate and extremism). An image with personal data (found on your PC and never sent, or seen by the AI) or an image the AI cannot check (no key, limit reached, provider down) is held for review, and the log says which case it was. For a GIF or a video, every frame is checked, so a harmful frame is blocked even when an earlier frame was only held. Moderators and trusted members are not checked. Optionally, GIFs and videos are checked from three frames each. Off by default; the AI can make mistakes.
- **AI help for personal data.** Moderation → Personal data protection → Ask the AI when the scan finds nothing sends images the local scan found nothing in (or could not finish) to your provider, which looks for anything that could locate or identify someone, such as a house number, a street sign, a plate, a document or a screen. Choose which images go: photos, screenshots, GIFs and videos (three frames), images the scan could not finish. Anything found is held for review; data the local scan finds is never sent. Off by default.
- **Simple and Advanced modes.** A switch at the top of the sidebar shows only the essentials (Simple, the default) or every page and setting (Advanced). Simple only hides things: every setting keeps its value, and opening a hidden page or search result switches to Advanced.
- **Control room.** A new Live page gathers what you touch during a stream: the pinned message, the current media with Skip, the waiting queue and the media waiting for your approval.
- **Foldable sidebar groups.** Click a group title (Get started, Live, Content…) to fold it; the open page always stays visible. Folding is ignored in the compact sidebar.
- **Quick fixes in Help.** Help starts with "Something does not work?": pick a symptom (media, message, sound, blank OBS source, bot offline) to see the steps and open the right page. Ask the AI now allows one question every 5 minutes and 10 a day. Relay itself applies this pace (a question that never reached the AI does not count), and the time left is shown under the button.
- **Cooldown notice.** When a member's message is ignored because it came before the delay between two messages, the bot replies in the channel with a mention, saying the delay and that faster messages are not shown, then deletes its reply after 10 seconds (Discord has no private reply to a plain message). At most once a minute per member and five times a minute in all, so a spammer or a raid cannot make the bot flood the channel. On by default; turn it off under Moderation → Spam and links → Tell the member. It needs Send Messages in the Relay channel, and the Discord check now says so when a delay is set.
- **No AI wait for one-word messages.** With Check messages before they appear on, a single short word without a link or mention (test, gg, salut) is shown at once instead of waiting for the AI; single-word insults are left to the word filter.
- **Member names in the decision log.** The Moderation log shows each author's Discord name instead of the raw ID when Relay can find it; the ID stays in the tooltip and still filters the log. Names are looked up for display only and never saved.
- **Windows widgets move together.** Dragging the unlocked media widget or the notification widget moves the other one (when shown) by the same amount, so both stay on the same screen.
- **Accent color presets.** Eight ready-made colors, a HEX field and a picker in Relay's style replace the grey Windows color window; the RGB fields are folded. The panel says whether button text will be dark or light.
- **Four designs.** Midnight and Ember (dark only), Dawn and Cloud, in the panel, the tray and the OBS and widget outputs.
- **Four languages.** Italiano, Português (Brasil), Polski and Türkçe, in the panel and the tray. Discord replies stay in English for these languages for now.
- **Skip from the tray.** The tray menu has a Skip button next to Panic, with your shortcuts shown when the panel has set them.
- **Moderation says why.** Each waiting item and each line of the decision log now explains in one sentence why Relay held, ignored or blocked it, and names the setting that changes it, with its current value (for example the delay between two messages). The explanation comes only from Relay's own rules; nothing leaves your PC.
- **AI assistant settings (groundwork).** Moderation → AI assistant lets you choose OpenAI or Anthropic, a fast or a more precise model, and hourly and daily request limits, and save one API key per provider in Windows Credential Manager. Keys are never shown again, never stored in the configuration file, and the diagnostic report only says whether one is saved. Content will only ever go to the provider you choose.
- **AI advice on held messages (off by default).** With Moderation → AI assistant → Advise me on held messages turned on, each text message held for review gets a short advisory line: clear problem, ambiguous or nothing spotted, with categories (harassment, threat, hate, sexual content, spam, filter evasion, self-harm) and a one-sentence reason. The advice never publishes, rejects or blocks anything, and "nothing spotted" is never presented as a guarantee. Only the message text is sent, without author, Discord IDs, channel or role names, or link details; a message that may hold personal data is never sent, even if the privacy scan is off for the stream. Trusted members and server staff are skipped, and nothing is asked while Relay is paused. The message tester also gets an Ask the AI button once a provider is chosen.
- **AI advice on held images (on demand).** Once a provider is chosen, each image or GIF waiting for review has an Ask the AI button. The answer adds two categories, private conversation and personal data, and is advice only: it never publishes, rejects or releases the item. Only the image itself is sent, never its file name, message text, author or Discord link. Relay first scans the bytes locally with every privacy rule on, even rules turned off for the stream, and sends nothing when the file holds a location or camera metadata, shows readable personal data, cannot be checked, is animated (only the first frame of an animated GIF, PNG or WebP can be checked on the PC), is not a PNG, JPEG, GIF or WebP, or is heavier than 7 MB. Videos and sounds are never sent either, and the panel says why nothing was sent.
- **AI settings assistant.** Once a provider is chosen, Moderation → Statistics, test a message, my presets has a box where you describe the settings you want in one sentence (for example "show GIFs directly, but review videos"). The AI proposes values for the settings a preset controls, and Relay shows which ones would change, in the same confirmation box as presets, with a warning in its own words when the proposal turns off manual review, the anti-doxxing scan, the deletion of blocked messages, the review of MEDIUM cases or a personal data category, or makes the scan less strict. Applying it only writes the settings the AI changed, so a setting you changed while waiting is kept, and an answer that arrives after you opened a preset is dropped. Nothing changes until you click Apply, and Undo stays available for 30 seconds. Only those settings and your sentence are sent, never channels, roles, members, your own words, the allowlist or private patterns; a sentence that may hold personal data is not sent. A proposal with a value Relay does not accept is refused as a whole.
- **AI troubleshooting help.** Help → Still stuck? has a box to ask the AI why something does not work (for example "why does my GIF not show on stream?"). Relay builds the report it sends itself, field by field: connection states, output counters, the moderation and privacy settings that hold or hide media, waiting items and the recent decision log as rule codes, never your token, links, paths, Discord IDs, channel, server or member names, or message text. Each finding is labelled by Relay: "From the report" only when the AI points to report lines that exist, "Guess" otherwise, followed by a first thing to try. The exact report sent is shown under the answer. A question that may hold personal data is not sent.
- **AI watch before publishing messages (off by default).** With Moderation → AI assistant → Check messages before they appear turned on, each text notification of a member that Relay would publish is first sent to the AI, waiting at most 6 seconds. When the AI sees a clear problem, the message waits in the review queue with the reason "flagged by the AI watch" and the AI's advice; an ambiguous message, a slow answer, a reached limit or any failure publishes it exactly as Relay would without AI. The AI can only hold a message: it never publishes, rejects, blocks or deletes anything, and it never sees messages that are already held or blocked, messages from trusted members or server staff, messages that may hold personal data, or anything while Relay is paused. Media are not watched. Each watched message costs one request and counts toward the hourly and daily limits.
- **AI connection test.** A Test the connection button sends one short fixed message (never Discord content) to the chosen provider and reports the model, response time and tokens used, or the cause of a failure in plain words: refused key, spend limit or credit reached, rate limit, outage, timeout. Relay retries a short outage once at most, never follows redirects, never keeps the provider's answers there (OpenAI `store: false`), pauses the AI for five minutes after three outages in a row, and stops calling a provider whose key was refused or whose spend limit was reached until a new key is saved or a test succeeds. The hourly and daily limits apply to every request, a retry included.

#### Changed

- **New logo.** The radar is replaced by a message card breaking out of a stream frame, in Relay green: in the taskbar, the notification area, the panel, the tray and the OBS outputs' placeholder images. A simplified drawing keeps it readable at 16 px.
- **Maximum media size.** Outputs → Media in OBS and Media widget have a Maximum media size (%) slider. A media fits in that share of the OBS source or the widget window (10 to 100 percent, 100 by default, so nothing changes until you lower it). With the current layout, a smaller media stays on the right and centred in the height; with an anchor, it is placed at that anchor.
- **Repeated media window.** With Ignore repeated media on, the same file, sticker or link is now ignored for 1 minute instead of 10, and you can set this time under Moderation → Spam and links → Repeated media window (5 seconds to 1 hour). The decision log says the time in use.
- **Clearer menu.** Pages are grouped by task: Get started (Overview, Discord, Discord commands), Live (Control room), Content (Messages, Media with History, Music, Sounds and reactions), Broadcast (Outputs), Safety (Moderation) and App (Settings, Help, Changelog, About). Overlay is now Outputs and Personalization is now Settings.
- **Outputs grouped by content.** The OBS message overlay switch, the music output note and the notification style moved to Outputs, under Media, Messages, Music and All outputs headings. The Relay channel is chosen only on the Discord page; Messages and Media no longer repeat it.
- **Simpler Moderation.** The page opens on four protection levels, each with one line saying what it does. Statistics, the message tester, your own presets and the AI suggestions are folded into one block, and the parts have plain names (Blocked words, Approve before showing, Personal data protection, Sanctions, During a stream) with an On/Off badge where one switch decides.
- **AI assistant in three steps.** Choose a provider, paste its key, then choose what the AI does; the model, limits and connection test are folded.
- **One entry per language.** English (UK) and English (India) are merged into English, and Español (Latinoamérica) into Español; a saved regional choice keeps its language.
- **Tray layout.** Discord and OBS are shown side by side, Open control panel is the main action, and the version is shown under Relay.
- Section numbers (01, 02…) and moderation letters (A, B…) are removed.
- Privacy and scan reasons in the queue and the decision log now use translated names (for example "postal addresses" or "incomplete check") instead of internal codes.

#### Fixed

- **Media shown twice in OBS.** The YouTube layer of the Relay Visual source also played Discord images, GIFs and videos, on top of the media layer: each one was drawn twice, and the author's name could appear twice at two sizes. The YouTube layer now leaves them to the media layer. It still shows the Now playing card of a Discord sound, but silently: the sound itself comes only from the Relay Audio source, so it is no longer played twice.
- After a language change, the media library's storage line and search field no longer keep the previous language until Relay restarts.

## [1.4.2] - 2026-09-28

### English

#### Changed

- A separator line in the panel sidebar now sets the page list apart from the language, theme and update controls.

The music jukebox uses much less of the daily YouTube Data API quota, so more Discord requests fit in a day before the quota runs out. Playback in OBS and the Windows widget is unchanged.

- **Fewer calls per search.** A music search now makes one YouTube search call instead of two, which halves its cost (about 100 quota units instead of 200). Results keep YouTube's relevance order; very recent uploads that are not among the most relevant results no longer appear.
- **Repeated searches are free.** The same search asked again within 6 hours reuses its first results instead of calling YouTube, so it costs no quota and answers at once. Case and extra spaces are ignored, the word filter still applies to every request, and the results are kept in memory only, until Relay restarts.
- **Pasted links are almost free.** A YouTube link posted alone in the music channel (watch, youtu.be, Shorts, embed, live or YouTube Music) now picks that exact video instead of searching its address as text. It costs about 1 quota unit instead of 100, and the video follows the same duration, spam and embedding rules as search results.

#### Fixed

- A message that took more than 20 seconds to prepare (for example a large audio file) no longer loses its notification and media when a GIF embed from the same message arrives later.
- **Blocked messages no longer leak a media.** When a Relay channel message with text and several attachments had a later part refused, an earlier attachment that was already checked could still reach the stream before the whole message was cancelled. The message is now cancelled in one step.
- **No more stuck media queue.** When one part of a message was still being prepared after 20 seconds while another part was already playing, the queue could stop releasing the remaining parts. They now play after the current one.
- **Media word filter respected at the final check.** With the word filter turned off for media, an attachment or sticker whose name contains a filtered word is no longer blocked by the last check.

## [1.4.1] - 2026-09-28

### English

#### Changed

- Replaced legacy branding in public metadata, documentation and test fixtures with inerthel.
- Messages and media now share one Relay channel, chosen on the Discord page. In that channel, text becomes a notification and each supported image, GIF, video, audio file and sticker goes to its output once, in Discord order, up to three attachments per message as before. A message with text and media shows one notification, then its media, without repeating the text on the media cards. Media alone never creates an empty notification. GIF links are shown as GIFs, not as notifications. Slash commands typed as text and Discord system messages are never shown as notifications. Stickers posted in the Relay channel use the sticker output. Music, reactions and the security trap keep their own channels.
- A message with text and media passes the moderation checks of both: spam, cooldown, duplicate and raid protection, and the word filter of every output it feeds. If one part is blocked, no part of the message reaches the stream.
- Existing settings move to the Relay channel automatically when only one channel was set, or when both were the same. When the media and message channels were different, both keep working as before until you choose which one to keep on the Discord page. `/relay channel` waits for that choice. `/relay status`, `/relay show`, the bot check and the diagnostic report show the Relay channel.
- The server owner and members allowed to moderate the server (Administrator, Manage Server or Manage Messages) are treated like trusted members: the delay between two messages, spam checks and manual review no longer hold back their media and messages. Privacy checks still apply to them.
- Size and crop has two YouTube outputs with a live preview. YouTube video in OBS scales the video from 10% to 400%, anchors it with margins and crops its edges. YouTube card in OBS scales and anchors the Now playing card, which follows the visible video. The YouTube source no longer follows the Media in OBS settings.
- OBS pages left open while Relay restarts, for example after an update, now reload themselves once they reconnect, so new settings and fixes apply without refreshing each Browser Source by hand.
- On OBS & widgets, Content scale now goes up to 400% so notifications and media can be made much larger on a 1080p or 4K canvas. With the Current layout anchor, the margin controls are greyed out because only an anchored position uses them.
- Animated GIFs from Tenor, Giphy and Klipy no longer wait for review only because text recognition reads their first frame. Every other privacy check still applies, and other animated images still wait for review when text recognition is on.
- The Commands page and Help describe the single Relay channel: `/relay channel` chooses it, `/relay lock` locks it, and the troubleshooting steps point to it.
- `/relay status` no longer shows a "Messages preparing" line that always read 0.
- The panel test files are no longer bundled into the application.
- Moderation → filter words has a "Weight of near matches" setting (1 to 100, 4 by default), which could only be changed by editing the configuration file.
- Settings still named after the removed speech feature are renamed in the configuration file, for example `notificationQueueLimit` instead of `ttsQueueLimit`. Relay converts older files on first start and keeps their values.
- Opening the bare local address (`http://localhost:<port>/`) now leads to Relay Visual instead of a page that required the secret.
- The application identifier is now `eu.inerthel.relay`. On first start, Relay copies its settings, decision log, media library, reactions and panel preferences from the previous `eu.stealthylabs.relay` folders, which stay in place, and moves the saved Discord, YouTube, OBS and Relay secrets to the new Windows Credential Manager entries.

#### Fixed

- Restored the Relay logo next to the `Relay` title in Windows taskbar previews; the app's own title bar still shows no icon.
- The skip shortcut and skip button now also end a sticker on screen instead of waiting for its display time.
- The control panel can show the reaction image preview and Tenor media again: its own security policy now matches the application's.
- HEVC video conversion finds FFmpeg installed with WinGet even when Windows has not refreshed the PATH yet, like audio trimming already did.

## [1.4.0] - 2026-09-26

### English

#### Added

- Panic button in the top bar, in the tray and on a global shortcut (Ctrl+Alt+P by default, configurable under Media → Keyboard controls). It clears every output, stops music and reactions, hides the media widget and pauses Relay. While paused, new media, messages, music requests and reactions are ignored instead of queued, and a banner offers Resume. Media awaiting moderation are kept.
- OBS & widgets can add Relay to OBS automatically through the OBS WebSocket server: connect with its password, pick a scene, and Relay creates or updates Relay Visual, Relay Audio and optionally Sounds and reactions, with audio routed to the OBS mixer. The password is stored in Windows Credential Manager and only 127.0.0.1 is contacted.
- Moderation presets (Relaxed, Standard, Strict, Big event) set the moderation switches in one click. Relay summarizes what will change first, offers Undo for 30 seconds, and never touches channels, your own words, private data or exempt roles. Your own presets can be saved on this PC.
- Built-in word lists in English and French (hate, scams, sexual content, harassment, verbal doxxing) matched like your own filter words, with per-word exceptions and hidden content unless you ask to see it.
- Message tester on the Moderation page: paste a message to see whether Relay would show, hold or block it and why, without publishing anything.
- Decision log: what Relay blocked, held, ignored, approved, expired or sanctioned over the last 7 days, with a last-24-hours summary and a filter by member. It stores the rule and the author's ID, never message content.
- Spam and link protection: a delay between two messages per member, raid protection that holds every media above a per-minute limit, repeated media detection, notification spam detection, and blocking of Discord invites, link shorteners and fake Discord or Steam domains.
- Trust settings: trusted roles and members skip manual review and spam checks, ignored members never reach the stream, and media from recent Discord accounts or recent arrivals can wait for review.
- Safety delay: every media can wait a few seconds in the queue before going on stream, with a countdown and a Reject button.
- Review queue actions: reject and delete the Discord message, reject and time out or ban the author, or approve and trust the author, plus keyboard shortcuts (A approve, R reject, arrows) and blurred previews of risky images.
- Sanctions: an optional translated warning when a message is blocked, automatic timeouts after repeated blocks, and Discord AutoMod synchronization of your words and the enabled lists.
- Live protection: an automatic live mode that applies a preset while OBS is streaming and restores your settings afterwards, a maximum length for videos and sounds on stream, and a loudness limiter for the audio card.
- Advanced editor for the variants and regular expressions of your filter words, and a choice of which channels (media, notifications, music) the word filter applies to.
- Personalization → Notification style chooses how notifications, the music card and media captions look on stream and in the widgets. Automatic follows the Relay design and theme; any design or the new Subtitle style (outlined text without a card) can be chosen instead, with a light, dark or automatic background, a live preview and a Back to automatic button.
- Signal design for the panel, the tray and the outputs: black and white, square edges, condensed capitals and the accent color as an on-air marker.
- Discord → Bot check reports whether Message Content Intent is enabled and which channel permissions each feature is missing (media, message notifications, music, security trap), with a button to re-invite the bot with the right permissions.
- Added YouTube music entries to History, with replay preserving the selected playback range.
- Added YouTube downloads from History through a Download menu in the panel offering MP3 audio or MP4 video, with automatic setup of missing download tools in Relay's per-user cache.
- Help → Troubleshooting has a Copy diagnostic button. The report lists versions, Discord and output status, widgets, enabled features and recent panel errors, and never includes the bot token, private OBS links, Discord IDs or the Windows user name.
- The tray shows a shortcut when media awaits moderation or Discord is not connected, and opens the control panel directly on the matching page.
- Overview offers a trial mode until Discord is connected: show the media widget or connect OBS, then send sample images, GIFs, videos, audio or notifications locally.
- The updater recognizes lettered release tags, such as `v1.3.6g`, and their signed installer names.
- Direct Tenor GIF links, including masked Discord links, are now accepted by Relay and allowed in OBS and Windows outputs.
- Results of history, widget and OBS actions appear in a notification visible from any page.
- Overview is now a setup checklist: connect the bot, invite it, choose the media channel, add the OBS sources and send a test. Each step shows its state and opens the matching setting. A short dashboard follows once setup is done.
- A new Discord page holds the bot connection, bot status and every channel (media, message notifications, music and the security trap) with one Save button. Other pages show their channel with a Change link.
- The sidebar shows a badge when Discord needs attention, when OBS is not connected, or when media awaits moderation. The status pills in the top bar open the matching page.
- OBS & widgets lists all three OBS links, including Sounds and reactions, with a short setup procedure. Output readiness includes the reactions source.

#### Changed

- Reduced the README to installation, usage, configuration and limitations; removed promotional copy and decorative content.

- The Moderation page now opens with presets, a last-24-hours summary and the tester, shows the review queue before the settings, and groups the settings into word filters, manual review, spam and links, anti-doxxing, sanctions and live protection, followed by the decision log.
- The compromised account trap can time out instead of kicking or banning, ignores roles you choose, logs its actions and sends its notice in the interface language.
- Unchecked media types in manual review can now be shown directly instead of being dropped.
- Message notifications are more compact: the card follows its text between 220 and 340 px instead of a fixed 400 × 88 px, the message is the main text and the name a smaller label, the avatar is smaller, the always-on presence dot is gone, and long messages stop at three lines.
- Notifications, the music card, media captions and the YouTube credit now share one style per design: same surfaces, borders, corners, fonts and motion. The YouTube equalizer and progress bar use the accent color instead of a fixed red. Edges stay opaque, so OBS draws no dark halo.
- Redesigned tray: a Live / Paused / Problem state pill, readable Discord and local relay lines, a Panic or Resume button, real widget icons with switches, and a clear Start with Windows switch.
- The bot invitation now also asks for Send Messages and Embed Links (music replies), plus Kick or Ban Members when the security trap is enabled.
- Music requests made while Relay is paused get a translated reply in Discord instead of queuing.
- Error messages from Relay are shown in the interface language by type (bot offline, invalid Discord ID, missing permission, file too large, network, and more). The original English message stays available as a tooltip.
- Settings now save automatically: switches and menus at once, typed values when you leave the field or press Enter. Save buttons are gone except for the Discord token, the YouTube API key and custom commands.
- Start with Windows is now also available under Personalization → System.
- Turning off Show author now hides media and music requester credits and displays messages as Anonymous with a neutral avatar and no guild tag. Changes apply to current and pinned messages in OBS and Windows.
- The sidebar groups pages by task: Get started, Content, Broadcast, Safety and App. Overlay is renamed OBS & widgets. Section numbers restart at 01 on each page, and Relay reopens the last page once setup is complete.
- Each setting now has a single home: the Music page links to Overlay for the OBS URL and media widget size, and the local port moved to Personalization → System.
- Keyboard focus is visible on every switch and text area, the active page is announced to screen readers, and language lists support arrow keys.
- Text size now also applies to history rows and module cards rendered after the change. The smallest text is 11px.
- The "OpenAI" and "Anthropic" designs are renamed "Graphite" and "Paper". Your saved choice is kept.
- Help steps open the pages they describe, the YouTube Data API guide is translated, and settings search now finds reaction, library and music queue settings.
- Settings search now exposes its expanded state to screen readers, and page navigation moves keyboard focus to the page title.
- Panel corners and layering are harmonized, and secondary labels use the theme’s readable text color.
- The music channel cleanup confirmation now uses a danger-styled button to distinguish message deletion.
- History and moderation thumbnails are larger, and YouTube entries show the video thumbnail. The media type label no longer covers the thumbnail.
- Creator and project links now point to `inerthel-agi` and the `inerthel-agi/relay` repository, including update checks and the changelog.

#### Fixed

- Updated rustls to 0.23.45 and rustls-webpki to 0.103.15 to address RUSTSEC-2026-0285.
- Bind Discord control commands to the server of the channel they control, including channel-lock restoration.
- Apply member admission rules to deferred GIF updates and preserve rejected or held decisions without counting accepted updates twice.
- Normalize URL authorities before checking invite, shortener and scam-domain filters.
- Require review for animated images when OCR has inspected only the first frame, while retaining detected privacy blocks.

- The bot invitation link opened from the panel was rejected as an unsupported link because its permission value no longer matched a fixed number. Any permission set is now accepted except Administrator.
- Fixed YouTube playback drift between OBS and the Windows widget by synchronizing both outputs to a shared server clock, including after delayed iframe readiness, buffering, and reconnects.
- A panic no longer empties the History list shown in the panel.
- Message notifications now slide in without bouncing and fade out instead of disappearing at once.
- Names shown on stream now go through the word filter; an offensive name appears as Anonymous.
- Notification messages flagged for review now wait in the review queue instead of disappearing silently.
- YouTube search results whose title or channel contains a filter word are no longer offered.
- The Discord bot check now requires Manage Messages whenever blocked messages are deleted, including by filter words alone.
- Filter words saved automatically no longer leave the Moderation page marked as unsaved.
- OBS outputs and widgets now follow the light theme and the chosen design instead of always drawing dark cards.
- Right-clicking Relay, its tray menu or a widget no longer opens the browser menu (Back, Refresh, Print). Text fields keep Copy and Paste.
- Incomplete image privacy scans now enter moderation when scanning is enabled, including when intermediate-risk review is disabled.
- Valid GIFs no longer enter moderation solely because the EXIF reader does not support GIF containers; image and OCR checks still apply.
- The Sounds and reactions output backs off between reconnection attempts and checks a moved server before switching port.

#### Removed

- Removed the unused speech page (`/tts`), two unused internal commands and obsolete configuration fields.

#### Security

- Local HTTP and WebSocket outputs now reject unexpected hosts and origins. Pages carrying the Relay output secret are no longer cached.

## [1.3.6] - 2026-09-08

### English

#### Added

- Reactions now play on Windows through an invisible audio receiver as well as OBS, using the reaction volume and working with the panel hidden.
- Added named output presets, persistent anchors/margins and local portrait, landscape, GIF, video, audio, notification and sticker previews/tests.
- Added a waiting queue with guarded removal, searchable/filterable history and output diagnostics.
- Added message pinning so the displayed message stays visible while media and music continue; queued messages resume after removal.
- Added a persistent local media library for importing, renaming, replaying and deleting copied images, GIFs and videos.
- Added music request limits and duplicate protection, with controls to move or remove pending tracks.
- Added sounds and reactions, disabled by default, with local tests, OBS and Windows output, named Discord channel and role selectors, cooldowns, and an optional protected instructions message.
- Added a sound excerpt chooser for sounds longer than 30 seconds, with preview, a 30-second limit and preservation of the original file.
- Added bounded FIFO reaction playback with up to 25 waiting requests, localized queue feedback, queue-full handling, and reaction-first behavior for the global skip shortcut; stopping clears all queued reactions.

#### Changed

- Separated module cards and controls with consistent spacing, and moved media channel routing to the Media page.
- Grouped reaction settings into collapsible sections while keeping sounds and playback controls accessible.
- Replaced the heavy native scrollbars with slim rounded handles, transparent tracks, and accent feedback on hover.
- Widened the expanded navigation sidebar so longer module names remain readable.
- Reaction names now stay in the control panel. OBS and Windows outputs display only the optional visual; sound-only reactions remain invisible.
- Refactored localization, preset storage, output layout, audio styles, HTTP handlers, Discord music and state caches/playback; separated tests across 19 Rust modules.
- Increased audio artwork to 112 px and reduced the audio card right margin to 4 px.
- Slightly enlarged audio artwork and increased text notification width, height, avatar and text size while retaining right alignment.
- Aligned fitted media and audio to the right edge with a 12 px margin; reduced the notification presence indicator, including its border.
- Removed unintended padding from audio metadata and shifted media and audio farther right within the available space.
- Reduced audio and notification card sizes and nudged fitted media right without changing the YouTube layout.
- Enlarged stickers, compacted sticker-only notifications, matched notification styling to audio cards, and attached author credits below fitted media or inside the audio card while preserving aspect ratios.
- Settings saves now preserve drafts on other pages and edits made while a save is pending. Automatic word filters save only the filter list; queued form saves use the latest saved configuration.
- Reduced introductory headings, removed empty disclosure columns, kept connection status visible in narrow windows, and distinguished unsaved, saved, and error feedback. Failed URL copies now offer a retry.
- Made the protected music welcome message optional, including when automatic cleanup is enabled. An empty field protects no welcome message; a supplied message is never deleted. Routing saves no longer recheck an unchanged music welcome message.

#### Fixed

- Fixed local reaction previews with Web Audio decoding and immediate audio activation; failed playback on one output no longer stops the others.
- Fixed excerpt preview playback with direct Web Audio decoding and immediate audio activation on click, preserving stop and cancellation behavior.
- Fixed reaction audio trimming when the desktop app inherits an outdated PATH by detecting installed WinGet FFmpeg; removed the widget move arrow and kept dragging on its content.
- Fixed embedded audio artwork in Now Playing and history with CSP-compatible image loading and bounded shared caching.

#### Removed

- Removed the Windows reaction widget; reactions use the OBS source.
- Removed Windows speech synthesis and voice controls. Message, emoji and sticker notifications, optional sounds and channel cleanup remain available; existing voice settings migrate to visual-only mode.

## [1.3.3] - 2026-09-04

### English

#### Added

- Added separate opt-in 24-hour cleanup for the media and TTS channels under Input routing, with optional protected welcome messages and five-minute history checks.
- Added automatic music channel cleanup with a verified, protected welcome message, 120-second result expiration, and a separate preview and confirmation for historical messages.
- Added a requester-only Loop button alongside Skip. Repeats keep the selected range, reuse the Discord card, and yield to queued work. Media volume help now explicitly includes YouTube.
- Added collapsible sections in Overview, Music, and Overlay, including size and crop controls.

#### Changed

- Updated release, changelog, and updater links to the official Relay repository.
- Notifications are wider and display up to six text lines, wrapping long text even without spaces.
- The OBS music credit shows the track title and Discord requester instead of the YouTube channel name.

#### Fixed

- Fixed scroll containment in short windows so expanded settings and their save buttons remain reachable; the sidebar scrolls independently.
- Automatically delete the triggering honeypot message after the kick or ban attempt, including when the action fails. Report deletion failures in the bot status.

#### Removed

- Removed the unused notification YouTube player and its styles; test-only Rust helpers are excluded from release builds.

## [1.3.2] - 2026-09-03

### English

#### Changed

- The native window title bar now follows the selected interface theme and its colors instead of switching between fixed light and dark styles.
- The GitHub profile link in the panel now points to the current creator profile.
- Automatic moderation now includes a configurable compromised account trap with a Discord channel selector, Kick or Ban action, and an English security notice for affected users.

#### Fixed

- Fixed inherited application icons lingering on the native title bar after theme changes.
- Restored the Relay logo and `Relay` title in Windows taskbar previews while keeping both hidden from the app's native title bar.

## [1.3.1] - 2026-08-17

### English

#### Added

- Added `/relay nuke <channel>` for Discord administrators to recreate a text or announcement channel and permanently remove its complete message history. Relay updates its configured channel references automatically; the bot requires **Manage Channels**.

#### Fixed

- Fixed the TTS notification card leaving an unused transparent strip in wide or resized Windows widgets.
- Fixed live media previews rendering transparency as white in the Relay panel. Previews now use a black backdrop without affecting OBS transparency.

#### Security

- Stopped using the vulnerable transitive versions `quick-xml 0.39.4` and `rustls-webpki 0.102.8` by moving file dialogs and Discord TLS to Windows-native backends.

## [1.3.0] - 2026-08-17

### English

#### Added

- Added YouTube music search in a configurable Discord channel, with up to 15 relevant results between one and five minutes long.
- Added 30-second previews, full-track playback, queueing, and a Now Playing card for OBS and the Windows widget.
- Added unified **Relay Visual** and **Relay Audio** OBS Browser Sources for media, stickers, TTS notifications, YouTube playback, Discord audio, and TTS voice. Legacy source URLs remain available during migration.
- Added History downloads, a configurable global media skip shortcut, and an English YouTube API setup guide in the Music panel and project documentation.
- Added the **Gridline** and **Lumen** interface designs, compact and dynamic sidebar layouts, and a collapsible design picker.
- Added CI dependency auditing for the Rust crate so known high-severity issues are checked on every verification run.
- Added an in-app **Changelog** page that shows bundled release notes in the current interface language.

#### Changed

- Media, TTS, and YouTube playback now share output scheduling so competing items do not play over one another.
- YouTube playback now respects widget sound settings and wakes the Windows audio output when needed.
- Media captions now stay compact and anchored to the active media or player card, with independent OBS and Windows widget visibility settings.
- Windows TTS notifications now use a denser toast (400×104 by default) that stays compact instead of stretching across leftover empty space. Existing generated 980×180 and 480×112 sizes migrate automatically; custom sizes are kept.
- Overlay move labels and preview copy now follow every Relay interface language, including Russian, Simplified Chinese, Korean, Japanese, and Indonesian.
- Language, theme, accent, and font-scale choices are now saved with Relay config and restored on launch, including tray-only and `--startup` sessions.

#### Fixed

- Fixed YouTube playback restoration after output wake-up and stop/start transitions.
- Fixed skip shortcut registration failures without discarding the previous shortcut.
- Fixed canceled media downloads leaving History actions in a busy state.
- Fixed GIF downloads accepting responses that were not GIF files.
- Fixed TTS and sticker Browser Sources dropping their queued items after a brief WebSocket drop.
- Fixed TTS and sticker outputs starting before the server granted the shared stage, which could overlap media or music.
- Fixed OBS TTS and sticker sources navigating blindly after a Relay port change. They now probe the new server first, with a timeout, so a failed load does not leave the Browser Source dead.
- Fixed the Windows media widget forgetting its position when hidden or locked immediately after a move.
- Fixed deferred Discord GIF updates ignoring privacy filter exemptions because role information was missing.
- Fixed YouTube track selections being discarded when the jukebox queue was already full.
- Fixed Relay reporting a Windows TTS failure after a successful visual fallback.

#### Security

- YouTube API keys are stored locally in Windows Credential Manager and are not shown again after saving.
- YouTube playback controls are restricted to the user who requested the track or a Discord administrator.
- Panel and overlay Tauri windows now use separate permission sets: overlay widgets can no longer invoke control-panel commands.
- Discord overlay URLs posted by the bot no longer include the local Relay secret. Short pages inject a page-local secret instead of a host-wide cookie.
- Unused privacy-threshold settings that no longer affected filtering were removed from the interface and config schema.

## [1.2.7] - 2026-08-14

### English

#### Added

- Added a local settings search bar with `Ctrl+K`, keyboard-accessible results, and page-aware Back and Forward controls.
- Added regional language choices with bundled SVG flags for English (US, UK, and India), French, German, Spanish, and Latin American Spanish while preserving the complete English, French, Spanish, and German dictionaries.
- Added Russian, Simplified Chinese, Korean, Japanese, and Indonesian choices with bundled SVG flags, translated core Relay controls, moderation, privacy protection, custom commands, and the system tray.
- Added eight locally bundled interface fonts: Bricolage Grotesque, DM Sans, Figtree, Inter, JetBrains Mono, Manrope, Poppins, and Space Grotesk. Font files never load from a remote service at runtime.

#### Changed

- Reorganized Moderation into three independent collapsible sections for automatic filtering, manual moderation, and anti-doxxing protection. The existing settings and save behavior are unchanged.
- Completed and corrected the Moderation translations in English, French, Spanish, and German, including protection profiles and input placeholders.

#### Fixed

- HEVC/H.265 Discord videos are now transcoded locally to an H.264-compatible cache when FFmpeg is available, allowing playback in Windows WebView2 widgets. Relay falls back to the original source if conversion cannot complete.

## [1.2.6] - 2026-08-14

### English

#### Fixed

- Discord messages blocked by an automatic filter word are now deleted when **Delete blocked Discord messages** is enabled and Relay has the **Manage Messages** permission, including when the local privacy scan is disabled.

## [1.2.5] - 2026-08-14

### English

#### Added

- Added a fully local anti-doxxing scanner with `SAFE`, `LOW`, `MEDIUM`, `HIGH`, and `CRITICAL` risk levels for Discord text, attachment names, images, and metadata.
- Added configurable Balanced, Strict, and Paranoid protection levels, per-category controls, an automatic block threshold, a local review option, an allowlist, and a private-data protection list.
- Added detection for email addresses, phone numbers, IP addresses, GPS coordinates, postal addresses, IBANs, validated payment cards, license plates, sensitive URLs, administrative-document signals, and user-protected strings.
- Added local Windows OCR and EXIF/GPS inspection for supported images without sending detected content to an external service.
- Added automatic deletion of Discord messages blocked by the selected privacy threshold when the bot has the **Manage Messages** permission.
- Added automatic filter words and phrases with configurable aliases, bounded regular expressions, cautious obfuscation handling, and role-based exemptions.
- Split the Commands page into **Default Commands** and **Custom Commands**, with up to 16 locally configured `/relay <name>` subcommands synchronized for the Relay bot.
- Added predefined Ban, Unban, Kick, Timeout, Remove timeout, Clear messages, Add role, Remove role, and Reply actions with required, optional, or fixed parameters and user, role, channel, and permission restrictions.

#### Security

- Privacy checks now run before sensitive Discord content can enter visible history, WebSocket or OBS output, Windows widgets, media caches, replay, or moderation approval paths.
- Image inspection now enforces trusted Discord CDN hosts, bounded downloads, file-signature checks, size, pixel and frame limits, concurrency limits, and timeouts.
- Privacy logs contain only the risk level, detected category codes, and action; detected addresses, contact details, OCR text, metadata values, and protected strings are not copied into logs.
- Custom moderation actions derive non-disableable Discord permissions, require a one-time 60-second confirmation, recheck authorization and role hierarchy before execution, and suppress mentions in predefined replies.
- Custom-command synchronization validates Discord's candidate schema before local persistence, restores the previous schema if persistence fails, and logs only the command name, action code, and sanitized outcome.

#### Fixed

- Postal addresses are now recognized across punctuation, unusual separators, obfuscated street types, and multiline layouts, including probable addresses without a postcode.
- Intermediate-risk media now uses the existing local moderation queue even when general manual moderation is disabled.
- OCR, malformed metadata, and image-decoder failures no longer interrupt Relay or expose scanned private values in errors.
- Custom Ban and Timeout commands now accept the camelCase action fields sent by the desktop editor while retaining compatibility with previously serialized snake_case fields.
- Custom Ban commands now accept either a current member or a verified Discord user ID that is not yet in the server, allowing a preemptive ban without bypassing hierarchy checks for present members.

## [1.2.1] - 2026-07-28

### English

#### Added

- Added `/relay status` to report the live Discord connection, local relay, OBS outputs, queues, and Windows widget state directly in Discord.
- Added `/relay test` for isolated local tests of media, audio, TTS, notifications, and stickers without posting to Discord or adding history entries.
- Added independent options to show up to 180 characters from the Discord media message in OBS and the Windows media widget.
- Added a **Start with Windows** toggle to the system tray; automatic launches now open Relay directly in the tray without showing the control panel.

#### Fixed

- Local media tests and incoming live media now wake the Windows widget before playback so their output is immediately visible.
- The Windows media widget now restores active media after hide/show and avoids WebView2 edge artifacts around videos.
- OBS media and audio outputs no longer overlap during playback.
- Discord invitation links now open reliably in the default web browser instead of File Explorer.

## [1.2.0] - 2026-07-21

### English

#### Added

- Added three selectable interface styles in Personalization: OpenAI, Anthropic, and Playful Neo-Brutalism, each with light and warm dark variants, keyboard focus, reduced-motion support, and narrow layouts.
- Added Discord guild tags and badges beside author names in TTS notifications when the user enables their primary guild identity.

#### Changed

- The system tray now follows the interface language, theme, and selected design, including translated live status and widget controls.

#### Fixed

- Interface text scaling now applies the selected factor exactly once, preventing oversized or overflowing OpenAI text above 100%.
- Narrow layouts now remain constrained to the viewport when using the Neo-Brutalism design.

## [1.1.23] - 2026-07-14

### English

#### Added

- Added an in-app update menu that can check for a new Relay release, then download and install it on confirmation.

#### Fixed

- Audio and video outputs now use an exclusive playback lease so music cannot start while a video is still playing.
- Relay now verifies every downloaded updater installer with an independently stored, pinned signing key before execution.
- Windows notification widgets now grow to the minimum height required by their content scale, preventing the card from disappearing at 135% and above.

## [1.1.22] - 2026-07-12

### English

#### Added

- Added an Output readiness center showing connection status for visual, audio, TTS, notification, and sticker outputs.
- Added separate OBS, preview, and Windows widget connection tracking for every output.
- Added isolated local output tests that never post to Discord or add entries to Relay history.

#### Changed

- The top-bar OBS source count now excludes internal previews and connection probes.

## [1.1.21] - 2026-07-12

### English

#### Added

- Added persistent live previews for media and notification output geometry in the Relay panel.
- Added a synchronized top-bar audio player with previous, pause/resume, and skip controls.

#### Fixed

- History now loads a first-frame thumbnail for videos and MP4 GIFs; the Relay logo is used only when loading fails.
- Discord stickers posted in the TTS channel now render in notification cards without speech synthesis.
- Notification content scaling keeps cards inside the viewport.

## [1.1.1] - 2026-07-12

### English

#### Added

- Added persistent resizing for the media and notification Windows widgets, with monitor-aware limits and an optional 16:9 ratio for media.
- Added independent crop controls (0–40% per side) and content scaling (50–200%) for media and notifications in OBS and Windows widgets.
- Added configurable Discord bot online status and activity text for custom, playing, listening, watching, and competing activities.
- Added optional media sound in the Windows widget and configurable notification sounds for the widget and OBS.

#### Changed

- Output geometry, crop, scale, and bot presence changes now apply live without reloading overlays or interrupting playback.
- Audio cards, notification cards, and author details now scale cleanly while preserving readable, bounded text.
- Remote artwork downloads now accept only approved HTTPS media hosts and safe redirects.

## [1.1.0] - 2026-07-12

### English

#### Added

- Added a dedicated Commands page with individual availability switches.
- Added `/relay clear` to delete messages from the configured Discord media and TTS channels without clearing Relay history.
- Added `/relay lock` as a reversible toggle for the configured Discord media channel.
- Added `/relay changelog <channel>` to post the latest release notes, fetched live from GitHub, into a chosen Discord channel.
- Preserved access for Discord administrators and moderation roles while a channel is locked.
- Stored channel permission snapshots locally so unlock restores the previous state.
- Added a dedicated `/stickers` OBS Browser Source with its own 50-item FIFO queue and configurable duration.
- Added Discord PNG, APNG, GIF, and Lottie sticker capture with bounded local caching and a safe visual fallback.
- Added visual rendering for Unicode, static custom, and animated custom emojis in TTS notifications.
- Added a "TTS voice" playback switch; when disabled, TTS messages become silent notifications.
- Added a configurable notification duration (1 to 60 seconds) controlling how long silent TTS notifications stay visible in OBS and the Windows widget.

#### Changed

- Reorganized the playback settings into collapsible categories: display durations, audio and TTS, display.
- Updated the Discord invitation URL with the permissions required for media reading, channel permission overwrites, and message cleanup.
- Documented command permissions in English, French, Spanish, and German.
- Messages containing an emoji now skip speech synthesis while preserving the author and message in the notification output.

#### Fixed

- TTS notifications now appear immediately and stay visible even when audio playback fails in OBS or the widget.
- Fixed visual emoji notifications blocking the following spoken TTS message.
- Added a synthesis timeout so a stalled Windows voice cannot freeze the global TTS queue.
- Added automatic fallback to the default Windows voice and preserved notifications when synthesis fails.
- Fixed `/relay clear` by requiring one explicit Discord channel and a message count from 1 to 1000.
- Fixed delayed Discord GIF embeds that arrived through partial message updates.
- Fixed favorite GIFs represented by Discord as thumbnail-only image embeds.
- Added support for direct thumbnail GIFs without a known GIF provider.

## [1.0.0] - 2026-07-12

### English

#### Added

- First public release of Relay for Windows.
- Discord media relay for OBS Browser Sources and Windows widgets.
- Separate media, audio, TTS, and notification outputs.
- Local moderation, playback controls, history, personalization, and multilingual interface.

[Unreleased]: https://github.com/inerthel-agi/relay/compare/v1.4.1...HEAD
[1.4.1]: https://github.com/inerthel-agi/relay/compare/v1.4.0...v1.4.1
[1.4.0]: https://github.com/inerthel-agi/relay/compare/v1.3.7...v1.4.0
[1.3.7]: https://github.com/inerthel-agi/relay/compare/v1.3.6...v1.3.7
[1.3.6]: https://github.com/inerthel-agi/relay/compare/v1.3.3...v1.3.6
[1.3.3]: https://github.com/inerthel-agi/relay/compare/v1.3.2...v1.3.3
[1.3.2]: https://github.com/inerthel-agi/relay/compare/v1.3.1...v1.3.2
[1.3.1]: https://github.com/inerthel-agi/relay/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/inerthel-agi/relay/compare/v1.2.7...v1.3.0
[1.2.7]: https://github.com/inerthel-agi/relay/compare/v1.2.6...v1.2.7
[1.2.6]: https://github.com/inerthel-agi/relay/compare/v1.2.5...v1.2.6
[1.2.5]: https://github.com/inerthel-agi/relay/compare/v1.2.1...v1.2.5
[1.2.1]: https://github.com/inerthel-agi/relay/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/inerthel-agi/relay/compare/v1.1.23...v1.2.0
[1.1.23]: https://github.com/inerthel-agi/relay/compare/v1.1.22...v1.1.23
[1.1.22]: https://github.com/inerthel-agi/relay/compare/v1.1.21...v1.1.22
[1.1.21]: https://github.com/inerthel-agi/relay/releases/tag/v1.1.21
[1.1.1]: https://github.com/inerthel-agi/relay/releases/tag/v1.1.1
[1.1.0]: https://github.com/inerthel-agi/relay/releases/tag/v1.1.0
[1.0.0]: https://github.com/inerthel-agi/relay/releases/tag/v1.0.0
