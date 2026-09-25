# Relay · Design system

## Direction

An offline-first Windows control surface with a calm technical tone: neutral surfaces, precise hierarchy and rounded geometry. Design names are neutral and do not reference other products or companies.

## Identity

- Primary asset: `gui/assets/relay-radar.png`
- Icon: white minimalist radar on a full black field
- Signature motion: audio waveform only while audio is playing
- Product name: Relay

## Tokens

All tokens are CSS custom properties on `:root` in `gui/panel.css`.

| Group | Tokens | Notes |
|---|---|---|
| Color | `--canvas`, `--surface`, `--surface-subtle`, `--ink`, `--muted`, `--faint`, `--line`, `--line-strong`, `--inverse`, `--inverse-ink`, `--success`, `--danger`, `--focus`, `--accent`, `--accent-ink` | Redefined by each design and theme |
| Now playing | `--player-*` | The now-playing card stays dark in every theme |
| Text scale | `--text-scale` | Set from the text size preference; every `font-size` is `calc(Npx * var(--text-scale))` |
| Radius | `--radius-xs` 4px, `--radius-sm` 6px, `--radius-md` 10px, `--radius-lg` 14px, `--radius-xl` 18px, `--radius-pill` 999px | Base components only; design blocks may override |
| Stacking | `--z-topbar` 4, `--z-menu` 20, `--z-popover` 24, `--z-search` 30, `--z-toast` 40 | |

Default graphite palette:

| Token | Light | Dark |
|---|---|---|
| `--canvas` | `#f4f4f1` | `#000000` |
| `--surface` | `#ffffff` | `#080808` |
| `--ink` | `#111110` | `#f5f5f1` |
| `--muted` | `#6d6d67` | `#a2a29a` |

## Designs

`graphite` (default), `paper`, `neo-brutalism`, `gridline` and `lumen`, each with light and dark variants. The choice is stored in `localStorage` under `relay-design` and shared with the tray panel. Earlier identifiers `openai` and `anthropic` migrate to `graphite` and `paper` on load.

## Interface

- 14 pages in five task groups: Get started (Overview, Discord, Help), Content (Messages, Media, Music, Sounds and reactions), Broadcast (OBS & widgets, History), Safety (Moderation, Commands), App (Personalization, Changelog, About).
- Overview is a setup checklist (bot, invitation, media channel, OBS sources, test) followed by a dashboard. Relay opens there until the bot and media channel are set, then on the last visited page (`relay-last-page`).
- The Discord page owns the connection and every channel select. Module pages show a `.channel-link` row that opens it.
- Internal links use `data-go-to-page` and optionally `data-go-to-target`, which opens enclosing `<details>` and highlights the target.
- Sidebar items may carry a `.navigation__badge`; top bar status pills are buttons that open their page.
- The tray attention row opens the panel on a page through `tray_open_control_panel({ page })`; Rust accepts only `overview`, `discord`, `moderation` and `overlay`, then emits `relay-open-page` to the main window.
- Until the bot is connected, Overview shows a trial block (`#trial-mode`) that tests the media widget or OBS without Discord.
- Backend errors are English. `setSaveState(..., "error", message)` shows a translated category from `gui/diagnostics.mjs` in other languages and keeps the original as `title`.
- The diagnostic report (`buildDiagnosticReport`) is plain English and passes every free-form value through `sanitizeDiagnosticText`.
- Sections are numbered from 01 within each page.
- Each setting has one home. Other pages link to it instead of repeating the field.
- Settings save automatically: toggles, menus and sliders shortly after they change, text and number fields on `change` (leaving the field or Enter). Secrets (Discord token, YouTube key) and custom commands keep an explicit button.
- Action results without a status line on the current page appear in the shared toast (`#toast`).
- 9 languages plus English (UK), English (India) and Spanish (Latin America) variants.
- Light and dark themes. First launch and "Restore defaults" follow the Windows color scheme.
- Images and GIFs use the configured timer. Video and audio play to completion.
- Skip is available from the panel and the global shortcut `Ctrl+Alt+S`.

## Accessibility

- Every interactive control shows a `--focus` ring on `:focus-visible`, including switches and `<summary>`.
- Body text uses `--ink` or `--muted`, which meet 4.5:1 on canvas and surface in every design. `--faint` is reserved for decorative numbers.
- Minimum font size: 11px at 100% text scale.
- The active sidebar item has `aria-current="page"`. Navigating moves focus to the page title.
- Language pickers support arrow keys, Home, End and Escape. Settings search is a combobox.
- `aria-label` and `title` strings are translated through `data-i18n-aria-label` and `data-i18n-title`.

## Constraints

- No third-party logos, wordmarks or implied affiliation.
- No decorative gradients, fake metrics or filler cards.
- Transparent overlay while idle.
