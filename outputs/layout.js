/* Shared, dependency-free placement for browser outputs. */
const RelayLayout = {
  anchors: ["legacy", "topLeft", "topCenter", "topRight", "center", "bottomLeft", "bottomCenter", "bottomRight"],
  position(width, height, viewportWidth, viewportHeight, geometry = {}) {
    const anchor = this.anchors.includes(geometry.anchor) ? geometry.anchor : "legacy";
    if (anchor === "legacy") return null;
    const x = Math.min(200, Math.max(0, Number(geometry.marginX) || 0));
    const y = Math.min(200, Math.max(0, Number(geometry.marginY) || 0));
    const right = Math.max(0, viewportWidth - width);
    const bottom = Math.max(0, viewportHeight - height);
    return {
      left: anchor.endsWith("Left") ? Math.min(x, right) : anchor.endsWith("Right") ? Math.max(0, right - x) : right / 2,
      top: anchor.startsWith("bottom") ? Math.max(0, bottom - y) : anchor === "center" ? bottom / 2 : Math.min(y, bottom),
    };
  },
  /**
   * Vertical move that takes a card off the boxes it would cover: the
   * smallest one that keeps it on screen, or none when nothing fits.
   */
  dodge(card, obstacles, viewportHeight, gap = 12) {
    const across = obstacles.filter((box) => box.left < card.right && box.right > card.left);
    const covers = (top) => across.some((box) => top < box.bottom && top + card.height > box.top);
    if (!covers(card.top)) return 0;
    const fitting = across
      .flatMap((box) => [box.bottom + gap, box.top - gap - card.height])
      .filter((top) => top >= 0 && top + card.height <= viewportHeight && !covers(top));
    if (fitting.length === 0) return 0;
    const best = fitting.reduce((closest, top) => (
      Math.abs(top - card.top) < Math.abs(closest - card.top) ? top : closest
    ));
    return best - card.top;
  },
};
