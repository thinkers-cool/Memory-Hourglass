import { describe, expect, it, vi } from "vitest";
import {
  formatImageViewerZoom,
  handleImageViewerRotateKey,
  handleImageViewerZoomKey,
} from "./imageViewerZoom";

describe("imageViewerZoom", () => {
  it("formats zoom percentage", () => {
    expect(formatImageViewerZoom(1)).toBe("100%");
    expect(formatImageViewerZoom(1.5)).toBe("150%");
  });

  it("handles zoom keyboard shortcuts", () => {
    const zoomIn = vi.fn();
    const zoomOut = vi.fn();
    const resetTransform = vi.fn();
    const actions = { zoomIn, zoomOut, resetTransform };

    expect(
      handleImageViewerZoomKey(
        { key: "=", preventDefault: vi.fn() },
        actions,
      ),
    ).toBe(true);
    expect(
      handleImageViewerZoomKey(
        { key: "+", preventDefault: vi.fn() },
        actions,
      ),
    ).toBe(true);
    expect(
      handleImageViewerZoomKey(
        { key: "-", preventDefault: vi.fn() },
        actions,
      ),
    ).toBe(true);
    const preventDefault = vi.fn();
    expect(
      handleImageViewerZoomKey({ key: "0", preventDefault }, actions),
    ).toBe(true);
    expect(zoomIn).toHaveBeenCalledTimes(2);
    expect(zoomOut).toHaveBeenCalledTimes(1);
    expect(resetTransform).toHaveBeenCalledTimes(1);
    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(
      handleImageViewerZoomKey(
        { key: "x", preventDefault: vi.fn() },
        actions,
      ),
    ).toBe(false);
  });

  it("handles rotate keyboard shortcuts", () => {
    const rotateClockwise = vi.fn();
    const rotateCounterClockwise = vi.fn();
    const actions = { rotateClockwise, rotateCounterClockwise };

    handleImageViewerRotateKey(
      { key: "r", shiftKey: false, preventDefault: vi.fn() },
      actions,
    );
    handleImageViewerRotateKey(
      { key: "R", shiftKey: true, preventDefault: vi.fn() },
      actions,
    );

    expect(rotateClockwise).toHaveBeenCalledTimes(1);
    expect(rotateCounterClockwise).toHaveBeenCalledTimes(1);
    expect(
      handleImageViewerRotateKey(
        { key: "x", shiftKey: false, preventDefault: vi.fn() },
        actions,
      ),
    ).toBe(false);
  });
});
