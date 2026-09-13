import { describe, expect, it, vi } from "vitest";
import {
  resolveTooltipCoords,
  resolveTooltipLayout,
  suggestPlacement,
} from "./tooltipPlacement";

function rect(
  overrides: Partial<DOMRect> & Pick<DOMRect, "top" | "left" | "right" | "bottom">,
): DOMRect {
  return {
    width: overrides.right - overrides.left,
    height: overrides.bottom - overrides.top,
    x: overrides.left,
    y: overrides.top,
    toJSON: () => ({}),
    ...overrides,
  } as DOMRect;
}

describe("tooltipPlacement", () => {
  it("suggests side and vertical placements near viewport edges", () => {
    vi.spyOn(window, "innerWidth", "get").mockReturnValue(1000);
    vi.spyOn(window, "innerHeight", "get").mockReturnValue(800);

    expect(suggestPlacement(rect({ top: 100, left: 20, right: 120, bottom: 130 }))).toBe(
      "right",
    );
    expect(suggestPlacement(rect({ top: 100, left: 900, right: 990, bottom: 130 }))).toBe(
      "left",
    );
    expect(suggestPlacement(rect({ top: 20, left: 400, right: 500, bottom: 50 }))).toBe(
      "bottom",
    );
    expect(suggestPlacement(rect({ top: 720, left: 400, right: 500, bottom: 790 }))).toBe(
      "top",
    );
    expect(suggestPlacement(rect({ top: 200, left: 400, right: 500, bottom: 230 }))).toBe(
      "top",
    );
  });

  it("falls back to clamped coordinates when every placement overflows", () => {
    vi.spyOn(window, "innerWidth", "get").mockReturnValue(200);
    vi.spyOn(window, "innerHeight", "get").mockReturnValue(200);
    const anchor = rect({ top: 0, left: 0, right: 180, bottom: 180 });
    const coords = resolveTooltipCoords("right", anchor, 160, 160);
    expect(coords.left).toBeGreaterThanOrEqual(8);
    expect(coords.top).toBeGreaterThanOrEqual(8);
  });

  it("returns null when anchor or size is missing", () => {
    expect(resolveTooltipLayout("top", undefined, 10, 10)).toBeNull();
    expect(resolveTooltipLayout("top", rect({ top: 0, left: 0, right: 10, bottom: 10 }), 0, 0)).toBeNull();
  });
});
