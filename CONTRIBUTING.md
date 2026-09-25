# Contributing to Relay

Relay is a Windows Tauri application. Read [the architecture map](docs/architecture.md) before changing Discord handling, local output routes, moderation, privacy, or widgets.

## Requirements

- Windows 10 or Windows 11
- Stable Rust with the Tauri prerequisites installed
- Node.js 20 or newer for the source-level interface tests
- OBS Studio and a Discord application only when manually testing those integrations

## Local setup

```powershell
git clone <repository-url>
cd relay-bot
Set-Location src-tauri
cargo tauri dev
```

The first run creates local configuration. Enter Discord credentials through Relay or a local `.env` file that is never committed. Do not place credentials in test fixtures, screenshots, issue reports, or pull requests.

## Validation

Run the checks relevant to your change before opening a pull request:

```powershell
# Rust core, from the repository root
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml

# Interface and Browser Sources, from the repository root
node --test gui/*.test.cjs overlay/*.test.cjs notifications/*.test.cjs stickers/*.test.cjs reactions/*.test.cjs
```

For a Windows, Discord, OBS, widget, OCR, FFmpeg, or updater change, follow the relevant rows in the [Windows smoke-test matrix](docs/windows-smoke-tests.md) and describe the observed result. Never attach private Browser Source URLs, tokens, personal media, or unredacted Discord messages.

## Change scope

- Keep changes small and focused. Avoid unrelated formatting or generated files.
- Preserve the local-first model. Relay does not add telemetry, cloud processing, or a remote media store without explicit maintainer approval.
- Preserve moderation and privacy gates. No new output, workflow, or integration may publish blocked content.
- Use local, fictional media and identifiers in tests. Do not use a real person's contact information, address, image metadata, or private string.
- Do not add dependencies, modify the lockfile, or alter release signing without explaining why in the pull request.

## Contribution workflow

1. Open or select an issue that describes the problem or proposal.
2. Create a branch such as `fix/widget-video-recovery` or `docs/contributor-guide`.
3. Add a focused regression test for changed behavior whenever practical.
4. Run the validation commands above.
5. Use a Conventional Commit message, for example `fix(privacy): preserve blocked media gate`.
6. Open a pull request with the problem, validation evidence, risks, and any remaining manual tests.

## Areas needing care

| Area | Review focus |
|---|---|
| `src-tauri/src/privacy.rs` | Data minimization, local-only analysis, false positives, fail-safe behavior |
| `src-tauri/src/bot.rs` | Discord permissions, attachment trust, message deletion and rate limits |
| `src-tauri/src/server.rs` | Localhost-only access, authorization and output compatibility |
| `src-tauri/src/state.rs` | Queue ordering, replay, moderation and cache boundaries |
| `gui/`, `overlay/`, `tts/`, `notifications/`, `stickers/` | Keyboard access, localization, output isolation and recovery |
| `src-tauri/src/updater.rs` | Official source validation and signed artifact verification |

## Reporting security issues

Do not open a public issue for a possible security or privacy vulnerability. Follow [SECURITY.md](SECURITY.md) instead.
