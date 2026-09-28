# Separate GIF duration and text selection

## Understanding

- Static images and animated GIFs use independent durations from 1 to 60 seconds.
- Existing installations copy their current image duration into the new GIF duration once.
- New installations default both durations to 8 seconds.
- GIF video payloads loop until the GIF timer expires; normal videos remain unchanged.
- Interface text is not selectable, while editable and copyable form fields remain selectable.
- Settings stay local and continue to apply live to connected outputs.

## Decision log

- Keep `displayDurationMs` for static images to preserve the existing contract.
- Add `gifDurationMs` rather than renaming both fields, minimizing migration risk.
- Migrate missing `gifDurationMs` from `displayDurationMs` during config loading and persist it.
- Use global `user-select: none`, then opt form fields and editable content back into text selection.
