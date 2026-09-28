# Relay moderation

## Understanding

- Every rule is optional. With default settings Relay relays media immediately, as before.
- Manual review holds media until a local approval. Images and GIFs share one type switch; video and audio have their own.
- Text rules (filter words, built-in word lists, link rules) run before anything reaches history, WebSocket clients, widgets or OBS.
- Approval, rejection and every setting live in the local Relay application only.

## Pipeline

1. **Honeypot.** A message in the trap channel triggers a translated DM, then a kick, ban or timeout, and deletion. Roles in `moderation.honeypotExemptRoleIds` only get the message deleted.
2. **Gate** (`src-tauri/src/moderation.rs`, `ModerationRuntime::evaluate`). It runs on the raw Discord message, before any scan, in this order:
   - ignored members are dropped;
   - trusted roles or members pass and skip manual review, cooldowns and spam checks; so do the server owner and members whose roles grant Administrator, Manage Server or Manage Messages (read from the Discord cache). Privacy rules still apply to them;
   - the per-member cooldown drops media or notifications that come too soon;
   - notification spam (mentions, capitals, emoji, repeated characters) is dropped;
   - media seen within 10 minutes is dropped as a duplicate;
   - recent accounts (from the Discord ID), recent members (`joined_at`) and raids (more than N media per minute) are held for review.

   The verdict is remembered per message ID for 10 minutes so the state layer can apply holds and trust.
3. **Text rules and privacy scan** (`privacy.rs`). `classify_text` uses custom concepts plus the enabled word lists (`moderation/packs.rs`, minus `packExclusions`) and link rules. Exempt roles and channels outside `filterScopes` skip every text rule. The image scan, OCR and EXIF checks are unchanged.
4. **Route** (`state.rs`, `review_route`):
   - a Block is dropped and logged;
   - a Review or a gate hold goes to the queue;
   - with manual review on, unchecked types are dropped, unless `reviewOnlySelected` lets them pass;
   - with a safety delay, media waits `safetyDelaySeconds` in the queue and publishes itself unless rejected.
5. **Notifications** flagged for review wait in a separate text queue when `reviewText` is on (the default). Before, they were dropped silently.

## Queue

- Memory only, 50 media and 50 messages. When full, the oldest item is evicted and the eviction is logged.
- Items older than `pendingExpiryMinutes` expire and are logged (0 keeps them).
- Turning manual review off keeps privacy reviews and gate holds.
- From the queue: approve, reject, reject and delete the Discord message, reject and time out or ban the author, approve and trust the author. Keyboard: A, R and the arrow keys.
- Risky images are blurred in the panel until hovered or selected.

## Sanctions

- `warnOnBlock` sends a translated DM at most every 10 minutes per member.
- `escalationBlocks` blocks within `escalationWindowMinutes` trigger a timeout of `escalationTimeoutMinutes`. This needs Moderate Members, which the bot check and the invite link then request.
- AutoMod synchronization creates or updates one keyword rule named "Relay word filter". It uses at most 1000 keywords of 60 characters, needs Manage Server, and is only used on demand.

## Live protection

- `livePreset`: every 15 seconds Relay asks OBS (WebSocket, 127.0.0.1) whether it is streaming. At stream start it saves the preset fields in `liveRestore` and applies the preset. At the end it restores them. An unreachable OBS changes nothing.
- `maxMediaSeconds` cuts long videos and sounds in the output pages.
- `loudnessLimiter` compresses the audio card through Web Audio. It only processes same-origin audio (`/media-audio`, `/media-cache`).

## Presets

`moderation/presets.rs` defines Relaxed, Standard, Strict and Big event over `PresetFields`. A preset never touches channels, custom words, private data, the allowlist or exempt roles. The panel previews the changed setting keys, applies, and offers Undo for 30 seconds. Personal presets are stored in `localStorage` (`relay-moderation-presets`, at most 12).

## Decision log

`moderation/log.rs` keeps at most 500 entries for 7 days in `moderation-log.json`, next to the config. Each entry holds the time, the channel kind, the action, a reason code and the author ID, never message text or media.

## Design decisions

- Chosen: central Rust queue and gate. Rejected: frontend-only interception, because OBS could receive media before the panel processes it.
- Chosen: volatile queue with expiry and a decision log. Rejected: disk persistence of pending media, to prevent unexpected delayed publication after a restart and unnecessary retention of user content.
- Chosen: local decisions, with Discord actions only when the streamer turns them on or clicks them. Rejected: Discord moderation commands for the queue, to keep approval private.
- Chosen: built-in lists matched through the existing normalizer. Rejected: a remote list service, to keep Relay offline-first.
