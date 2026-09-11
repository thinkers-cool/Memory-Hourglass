import { describe, expect, it } from "vitest";
import {
  computeAnchoredPopoverPlacement,
  computeFloatingMenuPlacement,
  resolveMeasuredMenuDimensions,
} from "./anchoredPopover";

describe("computeAnchoredPopoverPlacement", () => {
  it("opens below the anchor when there is enough space", () => {
    const placement = computeAnchoredPopoverPlacement({
      anchorLeft: 100,
      anchorTop: 100,
      anchorBottom: 132,
      boundaryLeft: 0,
      boundaryRight: 800,
      boundaryTop: 0,
      boundaryBottom: 600,
      preferredWidth: 236,
      preferredHeight: 248,
      margin: 8,
    });
    expect(placement.top).toBeGreaterThanOrEqual(132);
    expect(placement.width).toBe(236);
  });

  it("opens above the anchor when below space is tight", () => {
    const placement = computeAnchoredPopoverPlacement({
      anchorLeft: 100,
      anchorTop: 500,
      anchorBottom: 532,
      boundaryLeft: 0,
      boundaryRight: 800,
      boundaryTop: 0,
      boundaryBottom: 560,
      preferredWidth: 236,
      preferredHeight: 248,
      margin: 8,
    });
    expect(placement.top).toBeLessThan(500);
  });

  it("clamps width and left within the boundary", () => {
    const placement = computeAnchoredPopoverPlacement({
      anchorLeft: 760,
      anchorTop: 100,
      anchorBottom: 132,
      boundaryLeft: 0,
      boundaryRight: 800,
      boundaryTop: 0,
      boundaryBottom: 600,
      preferredWidth: 400,
      preferredHeight: 248,
      margin: 8,
    });
    expect(placement.width).toBeLessThanOrEqual(784);
    expect(placement.left + placement.width).toBeLessThanOrEqual(792);
  });

  it("enforces minimum width and height", () => {
    const placement = computeAnchoredPopoverPlacement({
      anchorLeft: 100,
      anchorTop: 100,
      anchorBottom: 110,
      boundaryLeft: 100,
      boundaryRight: 140,
      boundaryTop: 100,
      boundaryBottom: 130,
      preferredWidth: 120,
      preferredHeight: 120,
      margin: 8,
    });
    expect(placement.width).toBeGreaterThanOrEqual(168);
    expect(placement.height).toBeGreaterThanOrEqual(180);
  });
});

describe("computeFloatingMenuPlacement", () => {
  it("opens above and right-aligns to the anchor", () => {
    const position = computeFloatingMenuPlacement(
      { top: 500, right: 56, bottom: 544 },
      { width: 240, height: 320 },
      "top-end",
      8,
      { width: 1200, height: 800 },
    );
    expect(position.top).toBe(172);
    expect(position.left).toBe(8);
  });

  it("opens below and right-aligns to the anchor", () => {
    const position = computeFloatingMenuPlacement(
      { top: 24, right: 1200, bottom: 56 },
      { width: 240, height: 320 },
      "bottom-end",
      8,
      { width: 1200, height: 800 },
    );
    expect(position.top).toBe(64);
    expect(position.left).toBe(952);
  });
});

describe("resolveMeasuredMenuDimensions", () => {
  it("uses fallback dimensions when menu element is missing", () => {
    expect(resolveMeasuredMenuDimensions(null, 200, 160)).toEqual({
      width: 200,
      height: 160,
    });
  });

  it("uses measured dimensions when menu element is present", () => {
    const menu = document.createElement("div");
    Object.defineProperty(menu, "offsetWidth", {
      configurable: true,
      value: 180,
    });
    Object.defineProperty(menu, "offsetHeight", {
      configurable: true,
      value: 120,
    });
    expect(resolveMeasuredMenuDimensions(menu, 200, 160)).toEqual({
      width: 180,
      height: 120,
    });
  });
});
