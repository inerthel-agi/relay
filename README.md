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
.\Relay_1.4.0_x64-setup.exe
```

Or run `Relay_1.4.0_x64-portable.exe` directly.

To build from source with the prerequisites installed:

```powershell
git clone https://github.com/inerthel-agi/relay.git
cd relay/src-tauri
cargo tauri build --no-bundle
```

The executable is written to `src-tauri/target/release/relay.exe`.

## Usage

1. Open Relay and enter the bot credentials on the Discord page.
2. Invite the bot and select the media channel. Messages and music use separate optional channels.
3. On OBS & widgets, add the sources to OBS or copy their URLs into Browser Sources.
4. Keep Relay running and post media in the selected channel.

The Panic button clears outputs and pauses new playback. Resume restores playback admission.

See the [user guide](USAGE.md) for moderation, widgets, reactions and YouTube setup, and the [changelog](CHANGELOG.md) for release changes.

## Configuration

Edit settings in Relay. Configuration is stored in the application configuration directory; credentials use Windows Credential Manager.

| Setting | Type | Default | Effect |
|---|---|---|---|
| Local port | Integer | `4590` | HTTP and WebSocket server port; loopback only |
| Image / GIF / sticker / message duration | Seconds | `8` | Display time for each output type |
| Media volume | Percentage | `50` | Audio and video playback volume |
| Manual moderation | Boolean | Off | Holds selected media for approval |
| Reactions | Boolean | Off | Enables configured sounds and visuals |

## Limitations

- Windows only. OBS must run on the same computer.
- One media channel is watched per instance. History is limited to 50 entries and is lost on exit.
- OCR is limited to available French and English Windows language packs. Animated images require review when OCR has not inspected every frame; videos are not fully frame-scanned.
- Local output access does not isolate other programs running on the same computer.
- YouTube export may download external helpers. These use provider checksums; Relay installers use a separate pinned update signature.

See [SECURITY.md](SECURITY.md) for private vulnerability reporting and [docs/architecture.md](docs/architecture.md) for implementation details.

## License

[MIT](LICENSE).
