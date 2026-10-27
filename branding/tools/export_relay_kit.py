"""Exports the Relay logo kit: one-colour variants, PNGs, Windows .ico and web icons."""
import json
import os
import shutil
import struct
import subprocess
import sys

ROOT = sys.argv[1]  # branding folder
SKILL = "C:/Users/stealthy/.claude/skills/logo-design/scripts"
SVG = os.path.join(ROOT, "svg")
INK = "#111418"
GREEN = "#58B989"
GREEN_DEEP = "#2F9467"


def run(*args):
    result = subprocess.run([sys.executable, *args], capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(result.stdout + result.stderr)
    return result.stdout


def render(svg, out, width, height=None, bg=None, padding=None):
    os.makedirs(os.path.dirname(out), exist_ok=True)
    args = [f"{SKILL}/render_png.py", svg, "-o", out, "--width", str(width), "--height", str(height or width)]
    if bg:
        args += ["--bg", bg]
    if padding is not None:
        args += ["--padding", str(padding)]
    run(*args)


# One-colour variants of every master (black, white, deep green).
variants = os.path.join(SVG, "variants")
for master in ("relay-symbol", "relay-symbol-small", "relay-wordmark", "relay-horizontal", "relay-stacked"):
    run(f"{SKILL}/export_variants.py", os.path.join(SVG, master + ".svg"), "--out-dir", variants,
        "--title", "Relay logo", "--only", "black", "white", "mono", "--mono", GREEN_DEEP)

# PNGs for documents, slides and social posts.
png = os.path.join(ROOT, "png")
for name, sizes in (("relay-symbol", (512, 1024)), ("relay-symbol-on-dark", (512, 1024)), ("relay-app-icon", (512, 1024))):
    for size in sizes:
        render(os.path.join(SVG, name + ".svg"), os.path.join(png, f"{name}-{size}.png"), size)
for name, (w, h) in (("relay-horizontal", (1394, 512)), ("relay-horizontal-on-dark", (1394, 512)),
                     ("relay-stacked", (787, 812)), ("relay-stacked-on-dark", (787, 812)),
                     ("relay-wordmark", (1152, 420)), ("relay-wordmark-on-dark", (1152, 420))):
    render(os.path.join(SVG, name + ".svg"), os.path.join(png, f"{name}.png"), w, h)


def pack_ico(pngs, out):
    """ICO with PNG entries (Windows Vista and later read them at every size)."""
    entries = []
    for size, path in pngs:
        with open(path, "rb") as handle:
            entries.append((size, handle.read()))
    header = struct.pack("<HHH", 0, 1, len(entries))
    offset = 6 + 16 * len(entries)
    directory = b""
    blobs = b""
    for size, data in entries:
        dim = 0 if size >= 256 else size
        directory += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset + len(blobs))
        blobs += data
    with open(out, "wb") as handle:
        handle.write(header + directory + blobs)


# Windows icon: the small cut up to 32 px, the full symbol from 48 px.
windows = os.path.join(ROOT, "windows")
ico_pngs = []
# 32 px first: Tauri takes the first entry for the window and tray icon.
for size in (32, 16, 20, 24, 40, 48, 64, 96, 128, 256):
    source = "relay-app-icon-small.svg" if size <= 32 else "relay-app-icon.svg"
    out = os.path.join(windows, f"relay-{size}.png")
    render(os.path.join(SVG, source), out, size)
    ico_pngs.append((size, out))
pack_ico(ico_pngs, os.path.join(windows, "relay.ico"))

# Web and PWA icons.
web = os.path.join(ROOT, "web")
os.makedirs(web, exist_ok=True)
shutil.copyfile(os.path.join(SVG, "relay-app-icon-small.svg"), os.path.join(web, "favicon.svg"))
fav = []
for size in (16, 32, 48):
    out = os.path.join(web, f"favicon-{size}.png")
    render(os.path.join(SVG, "relay-app-icon-small.svg" if size <= 32 else "relay-app-icon.svg"), out, size)
    fav.append((size, out))
pack_ico(fav, os.path.join(web, "favicon.ico"))
render(os.path.join(SVG, "relay-app-icon.svg"), os.path.join(web, "apple-touch-icon.png"), 180)
render(os.path.join(SVG, "relay-app-icon.svg"), os.path.join(web, "icon-192.png"), 192)
render(os.path.join(SVG, "relay-app-icon.svg"), os.path.join(web, "icon-512.png"), 512)
# Maskable: full-bleed ink, the symbol inside the 80 % safe zone.
render(os.path.join(SVG, "relay-symbol-on-dark.svg"), os.path.join(web, "maskable-512.png"), 512, bg=INK, padding=0.16)
with open(os.path.join(web, "site.webmanifest"), "w", encoding="utf-8", newline="\n") as handle:
    json.dump({
        "name": "Relay", "short_name": "Relay", "theme_color": INK, "background_color": INK,
        "icons": [
            {"src": "icon-192.png", "sizes": "192x192", "type": "image/png"},
            {"src": "icon-512.png", "sizes": "512x512", "type": "image/png"},
            {"src": "maskable-512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable"},
        ],
    }, handle, indent=2)
    handle.write("\n")
with open(os.path.join(web, "head-snippet.html"), "w", encoding="utf-8", newline="\n") as handle:
    handle.write('<link rel="icon" href="/favicon.ico" sizes="48x48">\n'
                 '<link rel="icon" href="/favicon.svg" type="image/svg+xml">\n'
                 '<link rel="apple-touch-icon" href="/apple-touch-icon.png">\n'
                 '<link rel="manifest" href="/site.webmanifest">\n'
                 f'<meta name="theme-color" content="{INK}">\n')
print("done")
