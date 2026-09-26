# OLED theme and local help center

## Understanding

- Make the native Windows title bar and the application top bar visually uniform.
- Use true OLED black for the dark theme while keeping elevated controls readable.
- Preserve the existing light theme.
- Add a fifth bilingual Help page with collapsible installation and troubleshooting guides.
- Keep all instructions local and avoid changes to credentials, OBS URLs, queues, or bot behavior.

## Assumptions

- The dark canvas, sidebar, workspace, and top bar use `#000000`.
- Cards and inputs retain near-black elevated surfaces for hierarchy and accessibility.
- Official Discord and OBS links may be opened from the help content.
- The existing language switch updates every help heading and instruction.
- No new runtime dependency is introduced; the existing Windows crate gains only the DWM API feature.

## Approaches considered

1. **Windows DWM title colors plus CSS variables (selected).** Preserves native window controls while producing an exact title-bar match.
2. **Custom HTML title bar.** Fully styleable but duplicates window dragging, minimize, maximize, and close behavior.
3. **CSS-only application header.** Smallest change but cannot recolor the native Windows title bar shown in the screenshot.

## Final design

- A Tauri command applies the requested light or dark theme to the WebView window and Windows DWM caption, text, and border colors.
- Dark tokens use a pure-black canvas and sidebar/topbar; surfaces remain subtly elevated without gradients.
- The Help navigation entry opens a dedicated page with an introductory checklist and native `details` accordions.
- Guides cover Discord application creation, privileged intents, channel routing, three OBS Browser Sources, Windows widgets, and common failures.
- Accordions remain keyboard-accessible and require no JavaScript state management.

## Decision log

1. Keep native Windows decorations for reliability and recolor them through DWM.
2. Use pure black only for large background planes; controls remain distinguishable near-black surfaces.
3. Use native HTML disclosure elements because they are accessible, lightweight, and resilient.
4. Keep instructions inside the application so setup remains available offline.

## Validation

- Check JavaScript syntax and theme-command error handling.
- Verify light and OLED screenshots from the compiled WebView.
- Exercise every Help accordion in both languages.
- Run the complete JavaScript/Rust suites and strict Clippy.
- Rebuild the portable executable and NSIS installer without committing.
