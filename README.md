# Relay

Relay is a Windows desktop application that relays Discord media and messages to OBS Browser Sources and local widgets. Streamers configure channels, playback and moderation from a local control panel.

## Requirements

- Windows 10 or 11 with Microsoft Edge WebView2 Runtime.
- A Discord bot with Message Content Intent enabled and access to the selected channels.
- OBS Studio on the same computer for Browser Sources. Automatic setup uses obs-websocket 5 (OBS 28 or later).
- For source builds: Rust, Tauri CLI 2 and the Windows build tools described in [CONTRIBUTING.md](CONTRIBUTING.md).

## Install

Download the installer or portable executable from [Releases](https://github.com/inerthel-agi/relay/releases/latest).

Run the downloaded installer:

```powershell
.\Relay_1.5.2_x64-setup.exe
```

Or run `Relay_1.5.2_x64-portable.exe` directly.

To build from source with the prerequisites installed:

```powershell
git clone https://github.com/inerthel-agi/relay.git
cd relay/src-tauri
cargo tauri build --no-bundle
```

The executable is written to `src-tauri/target/release/relay.exe`.

## Usage

1. Open Relay and enter the bot credentials on the Discord page.
2. Invite the bot and select the Relay channel. Its text messages become notifications; its images, GIFs, videos, audio and stickers go to OBS. Music uses a separate optional channel.
3. On OBS & widgets, add the sources to OBS or copy their URLs into Browser Sources.
4. Keep Relay running and post media in the selected channel.

The Panic button clears outputs and pauses new playback. Resume restores playback admission.

See the [user guide](USAGE.md) for moderation, widgets, reactions and YouTube setup, and the [changelog](CHANGELOG.md) for release changes.

## Configuration

Edit settings in Relay. Configuration is stored in the application configuration directory; credentials use Windows Credential Manager.

| Setting | Type | Default | Effect |
|---|---|---|---|
| Local port | Integer | `4590` | HTTP and WebSocket server port; loopback only |
| Extra Relay, music and trap channels | Channel lists | Empty | Up to 10 more channels per feature, usually in other servers of the same bot, handled like the main channel of that feature (Advanced mode) |
| Image / GIF / sticker / message duration | Seconds | `8` | Display time for each output type |
| Media volume | Percentage | `50` | Audio and video playback volume |
| Manual moderation | Boolean | Off | Holds selected media for approval |
| Voice check | Boolean | Off | Transcribes voice messages, audio files and video sound on this PC, then checks the text for personal data and filter words before they play |
| Reactions | Boolean | Off | Enables configured sounds and visuals |

## Limitations

- Windows only. OBS must run on the same computer.
- One bot runs per instance. Extra channels must be in servers where that bot was added. Cleanup, welcome messages, lock, AutoMod and `/relay` commands only apply to the main channels and the Relay channel's server.
- History is limited to 50 entries and is lost on exit.
- OCR is limited to available French and English Windows language packs. Animated images require review when OCR has not inspected every frame; videos are not fully frame-scanned.
- The voice check needs FFmpeg 8 built with Whisper (for example `winget install Gyan.FFmpeg`; the essentials build lacks it) and a 60 MB model. It runs on the processor only. Sounds that take more than about two minutes to transcribe are held for review.
- Local output access does not isolate other programs running on the same computer.
- YouTube export may download external helpers. These use provider checksums; Relay installers use a separate pinned update signature.

See [SECURITY.md](SECURITY.md) for private vulnerability reporting and [docs/architecture.md](docs/architecture.md) for implementation details.

## License

[MIT](LICENSE).
