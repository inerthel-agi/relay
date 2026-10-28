# Relay logo guidelines

## 1. The logo

- **Idea**: a message card from Discord breaks out of the stream frame. It shows what Relay does: a message lands on the stream.
- **Versions**: horizontal (primary) · stacked · symbol · wordmark.
- **Small sizes**: from 32 px down, use the `-small` symbol (thicker frame, plain card). The app icon and favicon already do.
- **Files**:

| Folder | Content |
|---|---|
| `svg/` | Masters. `-on-dark` files are for dark backgrounds. |
| `svg/variants/` | One-colour versions: black, white, deep green. |
| `png/` | Transparent PNGs for documents, slides and posts. |
| `windows/relay.ico` | App icon, 16 to 256 px (small cut up to 32 px). |
| `web/` | Favicon, touch and PWA icons, `site.webmanifest`, `head-snippet.html`. |
| `presentation/` | Presentation board (HTML and one PNG per slide). |

## 2. Clear space

Let *x* be the width of the frame's band (24 units on the 256-unit symbol). Keep **2x** free on every side of the logo. The space scales with the logo.

## 3. Minimum size

| Version | Screen | Print |
|---|---|---|
| Horizontal | 120 px wide | 25 mm wide |
| Stacked | 64 px wide | 15 mm wide |
| Symbol | 16 px (small cut) | 6 mm |

## 4. Colour

| Name | Use | HEX | RGB | CMYK (approx.) |
|---|---|---|---|---|
| Relay green | Card on dark backgrounds; Relay's default accent | `#58B989` | 88 185 137 | 52 0 26 27 |
| Deep green | Card on light backgrounds (3.8:1 on white) | `#2F9467` | 47 148 103 | 68 0 30 42 |
| Ink | Frame and wordmark on light backgrounds; app icon tile | `#111418` | 17 20 24 | 29 17 0 91 |
| White | Frame and wordmark on dark backgrounds | `#FFFFFF` | 255 255 255 | 0 0 0 0 |

CMYK values are converted from RGB: check them with the printer. Pick a Pantone match from a printed guide.

**Approved pairs**: ink and deep green on white or light grey · white and Relay green on ink or near-black · one colour (black, white or deep green) when only one colour is available. On a green background, use the white one-colour version. On a photo, use a calm area or the app icon tile.

## 5. Typography

- **Wordmark**: drawn for Relay as outlines (geometric, 28-unit stems). Never retype it in a font.
- **Panel**: Bahnschrift, then Segoe UI Variable Text. **Outputs**: the bundled fonts (DM Sans, Inter, Manrope, Poppins, Space Grotesk, JetBrains Mono, all under open licences).

## 6. Don'ts

Don't stretch, squash or rotate the logo · don't close the gap between the card and the frame · don't recolour it outside the palette · don't add shadows, outlines, gradients or glows · don't move or resize the parts of a lockup · don't put the ink version on a dark background · don't retype the wordmark.

## 7. Notes

- Masters contain only filled paths: no live text, strokes, masks or filters.
- Lockups place the wordmark with a single `translate`/`scale` group; flatten it in a vector editor if a tool requires it.
- A trademark search has not been done. Run one before a public launch.
- Rebuild: `python tools/build_relay_logo.py svg` writes the masters; `python tools/export_relay_kit.py .` writes the variants, PNGs and icons (it uses the logo-design skill's scripts, path set at the top of the file).
