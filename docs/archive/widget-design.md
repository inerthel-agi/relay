# Movable display widget

## Understanding

- Add a second transparent Tauri window for the local display widget.
- Keep the OBS Browser Source independent and unchanged.
- Allow the 640x360 widget to move across Windows monitors.
- Show a blue dashed frame and a move badge while unlocked.
- Make the widget click-through while locked.
- Persist visibility, lock state, and the last valid position.
- Keep the widget local, always on top, and absent from the taskbar.

## Assumptions

- The widget is hidden on first launch.
- Its size is fixed at 640x360.
- A position outside all connected displays falls back to the primary display.
- The existing authenticated overlay page is reused with `widget=1`.

## Decision log

1. Use a dedicated Tauri WebView window. Native Rust rendering and a detached preview were rejected because they duplicate overlay behavior or cannot provide reliable click-through behavior.
2. Persist widget state in the existing local `config.json`. No new dependency or secret is introduced.
3. Expose show/hide and lock/unlock from both the control panel and tray menu.
4. Treat closing the widget as hiding it; the bot and local relay continue running.

## Validation

- Unit-test configuration defaults and persistence.
- Validate JavaScript syntax and Rust with tests and strict Clippy.
- Verify dragging, click-through lock, remembered position, multi-monitor fallback, and release startup.
- Rebuild the portable executable and NSIS installer without committing.
