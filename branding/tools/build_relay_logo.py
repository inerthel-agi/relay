"""Builds the Relay logo kit (direction C, "the card that leaves the frame") as clean SVG.

Every shape is a filled path: no strokes, masks, filters or live text.
"""
import math
import os
import sys

OUT = sys.argv[1] if len(sys.argv) > 1 else "out"
os.makedirs(OUT, exist_ok=True)

INK = "#111418"
WHITE = "#FFFFFF"
GREEN = "#58B989"  # Relay green, the panel's default accent; on dark backgrounds
GREEN_DEEP = "#2F9467"  # same hue, 3.8:1 on white; on light backgrounds


def n(v):
    v = round(v, 2)
    return str(int(v)) if v == int(v) else str(v)


def rrect(x, y, w, h, r):
    """Rounded rectangle, clockwise."""
    return (f"M{n(x + r)} {n(y)} H{n(x + w - r)} A{n(r)} {n(r)} 0 0 1 {n(x + w)} {n(y + r)} "
            f"V{n(y + h - r)} A{n(r)} {n(r)} 0 0 1 {n(x + w - r)} {n(y + h)} H{n(x + r)} "
            f"A{n(r)} {n(r)} 0 0 1 {n(x)} {n(y + h - r)} V{n(y + r)} A{n(r)} {n(r)} 0 0 1 {n(x + r)} {n(y)} Z")


def circle(cx, cy, r):
    return (f"M{n(cx)} {n(cy - r)} A{n(r)} {n(r)} 0 1 1 {n(cx)} {n(cy + r)} "
            f"A{n(r)} {n(r)} 0 1 1 {n(cx)} {n(cy - r)} Z")


def poly(points):
    head, *rest = points
    return "M" + " L".join(f"{n(x)} {n(y)}" for x, y in [head, *rest]) + " Z"


# ---------------------------------------------------------------- symbol
# Grid: 256 canvas, unit 4. The stream frame is a 24-unit band (outer radius 40,
# inner 16); it stops 12 units short of the message card, which breaks out of
# its lower-left corner. The card carries an avatar and a line of text as holes.
DX, DY = 2, 6  # optical centring: centred across, a little above centre down


def frame(stroke=24, gap=12, card=(16, 144, 144, 64)):
    """Frame band, open where the card crosses it."""
    cx, cy, cw, ch = card
    outer = (32, 28, 236, 188)
    ro = 40
    ri = ro - stroke
    x0, y0, x1, y1 = outer
    stem_end = cy - gap  # left side stops above the card
    bar_end = cx + cw + gap  # bottom side stops right of the card
    return (f"M{n(x0 + DX)} {n(stem_end + DY)} V{n(y0 + ro + DY)} "
            f"A{ro} {ro} 0 0 1 {n(x0 + ro + DX)} {n(y0 + DY)} H{n(x1 - ro + DX)} "
            f"A{ro} {ro} 0 0 1 {n(x1 + DX)} {n(y0 + ro + DY)} V{n(y1 - ro + DY)} "
            f"A{ro} {ro} 0 0 1 {n(x1 - ro + DX)} {n(y1 + DY)} H{n(bar_end + DX)} "
            f"V{n(y1 - stroke + DY)} H{n(x1 - ro + DX)} "
            f"A{ri} {ri} 0 0 0 {n(x1 - stroke + DX)} {n(y1 - ro + DY)} V{n(y0 + ro + DY)} "
            f"A{ri} {ri} 0 0 0 {n(x1 - ro + DX)} {n(y0 + stroke + DY)} H{n(x0 + ro + DX)} "
            f"A{ri} {ri} 0 0 0 {n(x0 + stroke + DX)} {n(y0 + ro + DY)} V{n(stem_end + DY)} Z")


CARD = (16, 144, 144, 64)


def card(holes=True, box=CARD):
    x, y, w, h = box
    d = rrect(x + DX, y + DY, w, h, 20)
    if holes:
        d += " " + circle(48 + DX, 176 + DY, 14)
        d += " " + rrect(80 + DX, 170 + DY, 58, 12, 6)
    return d


def symbol_paths(frame_fill, card_fill, small=False):
    if small:
        f = frame(stroke=32, gap=16)
        c = card(holes=False)
    else:
        f = frame()
        c = card()
    return (f'<path id="frame" fill="{frame_fill}" d="{f}"/>\n'
            f'  <path id="card" fill="{card_fill}" fill-rule="evenodd" d="{c}"/>')


# ---------------------------------------------------------------- wordmark
# Geometric letters on a 28-unit stem, cap height 160, x-height 120 (top at 40),
# descender to 210. Round letters overshoot by 2; the y's vertex by 4.
W = 28


def letter_R(x):
    stem = poly([(x, 0), (x + W, 0), (x + W, 160), (x, 160)])
    bowl = (f"M{n(x)} 0 H{n(x + 56)} A46 46 0 0 1 {n(x + 56)} 92 H{n(x)} Z "
            f"M{n(x + W)} 28 V64 H{n(x + 56)} A18 18 0 0 0 {n(x + 56)} 28 Z")
    leg_w = W * math.sqrt(2)
    leg = poly([(x + 2.4, 92), (x + 2.4 + leg_w, 92), (x + 70.4 + leg_w, 160), (x + 70.4, 160)])
    return [stem, bowl, leg], x + 70.4 + leg_w


def ring_point(cx, cy, r, deg):
    a = math.radians(deg)
    return cx + r * math.cos(a), cy + r * math.sin(a)


def letter_e(x):
    ro, ri = 62, 62 - W
    cx, cy = x + ro, 100
    bar_top, bar_bottom = cy - 14, cy + 10
    # Ring from the lower terminal (40°) clockwise round to the bar's underside.
    t_out = ring_point(cx, cy, ro, 40)
    t_in = ring_point(cx, cy, ri, 40)
    e_out = (cx + math.sqrt(ro ** 2 - (bar_bottom - cy) ** 2), bar_bottom)
    e_in = (cx + math.sqrt(ri ** 2 - (bar_bottom - cy) ** 2), bar_bottom)
    ring = (f"M{n(t_out[0])} {n(t_out[1])} A{ro} {ro} 0 1 1 {n(e_out[0])} {n(e_out[1])} "
            f"L{n(e_in[0])} {n(e_in[1])} A{ri} {ri} 0 1 0 {n(t_in[0])} {n(t_in[1])} Z")
    # The bar stops on the circle at its top edge, so no corner pokes out of the round.
    bar_right = cx + math.sqrt(ro ** 2 - (cy - bar_top) ** 2)
    bar = poly([(cx - ri - 4, bar_top), (bar_right, bar_top), (bar_right, bar_bottom), (cx - ri - 4, bar_bottom)])
    return [ring, bar], cx + ro


def letter_a(x):
    ro, ri = 62, 62 - W
    cx, cy = x + ro, 100
    ring = circle(cx, cy, ro) + " " + (f"M{n(cx)} {n(cy - ri)} A{ri} {ri} 0 1 0 {n(cx)} {n(cy + ri)} "
                                       f"A{ri} {ri} 0 1 0 {n(cx)} {n(cy - ri)} Z")
    stem = poly([(cx + ro - W, 40), (cx + ro, 40), (cx + ro, 160), (cx + ro - W, 160)])
    return [ring, stem], cx + ro


def letter_l(x):
    return [poly([(x, 0), (x + W, 0), (x + W, 160), (x, 160)])], x + W


def letter_y(x):
    hw = W / math.sin(math.atan(2))  # horizontal width of a 1:2 diagonal
    right = poly([(x + 124 - hw, 40), (x + 124, 40), (x + 39, 210), (x + 39 - hw, 210)])
    left = poly([(x, 40), (x + hw, 40), (x + 31.3 + 46.4, 132.7), (x + 62, 164)])
    return [left, right], x + 124


def wordmark_paths(fill):
    parts = []
    x = 0
    # Space after each letter, set by eye: straight sides get more room than round ones.
    advance = {"R": 14, "e": 20, "l": 18, "a": 14}
    for glyph, fn in (("R", letter_R), ("e", letter_e), ("l", letter_l), ("a", letter_a), ("y", letter_y)):
        shapes, right = fn(x)
        parts.extend(shapes)
        x = right + advance.get(glyph, 0)
    width = x
    d = " ".join(parts)
    return f'<path id="wordmark" fill="{fill}" fill-rule="nonzero" d="{d}"/>', width


WORD_W = None


def svg(view_w, view_h, title, body):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {n(view_w)} {n(view_h)}" '
            f'width="{n(view_w)}" height="{n(view_h)}" role="img" aria-labelledby="title">\n'
            f'  <title id="title">{title}</title>\n  {body}\n</svg>\n')


def write(name, content):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as handle:
        handle.write(content)


# Symbol masters.
write("relay-symbol.svg", svg(256, 256, "Relay logo", symbol_paths(INK, GREEN_DEEP)))
write("relay-symbol-on-dark.svg", svg(256, 256, "Relay logo", symbol_paths(WHITE, GREEN)))
write("relay-symbol-small.svg", svg(256, 256, "Relay logo", symbol_paths(INK, GREEN_DEEP, small=True)))
write("relay-symbol-small-on-dark.svg", svg(256, 256, "Relay logo", symbol_paths(WHITE, GREEN, small=True)))

# Wordmark.
word, word_w = wordmark_paths(INK)
write("relay-wordmark.svg", svg(word_w, 210, "Relay", word.replace('d="', 'd="', 1)))
word_dark, _ = wordmark_paths(WHITE)
write("relay-wordmark-on-dark.svg", svg(word_w, 210, "Relay", word_dark))

# Horizontal lockup: wordmark cap height = 0.7 × 160; baseline on the frame's bottom edge.
scale = 0.7
gap = 36
symbol_right = 238 + DX  # frame outer right edge
word_x = symbol_right + gap
baseline = 188 + DY
word_y = baseline - 160 * scale
lock_w = word_x + word_w * scale + 18
for suffix, frame_fill, card_fill, ink in (("", INK, GREEN_DEEP, INK), ("-on-dark", WHITE, GREEN, WHITE)):
    body = (symbol_paths(frame_fill, card_fill) + "\n  "
            + f'<g transform="translate({n(word_x)} {n(word_y)}) scale({scale})">'
            + wordmark_paths(ink)[0] + "</g>")
    write(f"relay-horizontal{suffix}.svg", svg(lock_w, 256, "Relay", body))

# Stacked lockup: symbol on top, wordmark centred below.
st_scale = 0.6
st_word_w = word_w * st_scale
st_w = max(256, st_word_w + 48)
sym_x = (st_w - 256) / 2
st_word_x = (st_w - st_word_w) / 2
st_word_y = 256 + 8
st_h = st_word_y + 210 * st_scale + 16
for suffix, frame_fill, card_fill, ink in (("", INK, GREEN_DEEP, INK), ("-on-dark", WHITE, GREEN, WHITE)):
    body = (f'<g transform="translate({n(sym_x)} 0)">' + symbol_paths(frame_fill, card_fill) + "</g>\n  "
            + f'<g transform="translate({n(st_word_x)} {n(st_word_y)}) scale({st_scale})">'
            + wordmark_paths(ink)[0] + "</g>")
    write(f"relay-stacked{suffix}.svg", svg(st_w, st_h, "Relay", body))

# App icon: the symbol (white frame, Relay green card) on an ink tile.
for name, small in (("relay-app-icon.svg", False), ("relay-app-icon-small.svg", True)):
    icon_scale = 0.74 if not small else 0.8
    offset = (256 - 256 * icon_scale) / 2
    body = (f'<rect width="256" height="256" rx="56" fill="{INK}"/>\n  '
            f'<g transform="translate({n(offset)} {n(offset)}) scale({icon_scale})">'
            + symbol_paths(WHITE, GREEN, small=small) + "</g>")
    write(name, svg(256, 256, "Relay", body))

# Full-bleed square for places that round the corners themselves (panel and tray logo).
body = (f'<rect width="256" height="256" fill="{INK}"/>\n  '
        f'<g transform="translate(33.28 33.28) scale(0.74)">' + symbol_paths(WHITE, GREEN) + "</g>")
write("relay-app-icon-square.svg", svg(256, 256, "Relay", body))

print(f"wordmark width {n(word_w)}, horizontal {n(lock_w)}x256, stacked {n(st_w)}x{n(st_h)}")
