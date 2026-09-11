import { describe, expect, it, vi } from "vitest";
import { formatImageViewerZoom, handleImageViewerZoomKey } from "./imageViewerZoom";

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
});
