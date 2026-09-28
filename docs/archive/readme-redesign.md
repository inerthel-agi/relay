# README redesign

## Understanding

- Rebuild the README in English for Windows streamers, OBS users, moderators, and self-hosting users.
- Combine product-first technical documentation with a strong visual showcase.
- Preserve the product name `Relay` and the existing radar logo.
- Keep every claim grounded in the current application.
- Keep sensitive Discord content, credentials, private URLs, and identifiable users out of visuals.
- Link to `USAGE.md` for the long-form walkthrough instead of duplicating it.

## Assumptions

- GitHub is the primary rendering target.
- Two generated raster assets are sufficient: one hero and one workflow illustration.
- Generated assets live in `assets/readme/` and remain easy to replace.
- The README remains plain Markdown with small centered HTML fragments where useful.
- The existing `gui/assets/relay-radar.png` remains the canonical logo.

## Decision log

- Chose a hybrid product and visual-showcase approach over a minimal technical README or a marketing-only page.
- Chose English-only documentation for international reach.
- Chose abstract, source-faithful visuals instead of fake screenshots or copied Discord/OBS interfaces.
- Chose a concise README plus `USAGE.md` for detailed setup and troubleshooting.

## Final design

The README opens with the Relay identity, a wide generated hero, a concise value proposition, and navigation links. A second generated visual explains the Discord-to-Relay-to-OBS flow. The remaining sections cover outcomes, supported media, installation, quick start, dedicated OBS sources, moderation, local security, architecture, configuration, development, tests, project structure, and license. Visuals avoid embedded text so the Markdown remains accessible, searchable, and maintainable.
