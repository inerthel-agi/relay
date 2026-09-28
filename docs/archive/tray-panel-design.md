# Custom system tray panel

## Understanding

- Replace the native Windows tray menu with a custom Tauri WebView panel.
- Match the control panel's dark, restrained visual language and radar identity.
- Keep the Discord bot, OBS sources, local server, and stored credentials unchanged.
- Expose relay status and the existing media/notification widget controls.
- Open from either primary or secondary tray-icon click and close on focus loss.
- Keep the panel local, lightweight, single-instance, and absent from the taskbar.

## Assumptions

- The tray panel uses a fixed dark theme because it appears outside the main app context.
- The panel opens above the taskbar near the click position and is clamped to the active monitor.
- A double click does not launch a second action after the panel has opened.
- No new dependency is required.

## Approaches considered

1. **Dedicated Tauri WebView panel (selected).** Fully styleable, reuses existing commands and state, and remains consistent with the main interface.
2. **Native Windows menu.** Smallest implementation, but Windows owns its rendering and prevents the requested redesign.
3. **Open the main control panel from the tray.** Avoids another window, but is too large and does not behave like a tray surface.

## Final design

- A compact 336 x 430 borderless window with the radar mark, Relay title, and live Discord/server indicators.
- Two grouped widget rows expose show/hide and lock/unlock for media and TTS notifications.
- A primary action opens the control panel; a quiet destructive action exits the application.
- The tray window refreshes state whenever it becomes visible and after every action.
- Focus loss hides the window. Closing it also hides it instead of stopping the relay.

## Decision log

1. Use a WebView rather than a native menu because visual control is the primary requirement.
2. Keep all actions as Tauri commands in the existing Rust entry module to avoid a new backend abstraction.
3. Use separate HTML, CSS, and JavaScript assets so the existing CSP remains strict and inline code is unnecessary.
4. Support both left and right click because the user delegated the interaction choice and requested direct access.

## Validation

- Check all modified JavaScript with `node --check`.
- Run focused frontend tests, Rust tests, and strict Clippy.
- Verify tray opening, focus-loss dismissal, widget actions, single-instance behavior, and release startup.
- Rebuild the portable executable and NSIS installer without committing.
