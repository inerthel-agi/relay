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
};
