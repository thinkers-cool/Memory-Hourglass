import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useZoomableMedia } from "./useZoomableMedia";

describe("useZoomableMedia", () => {
  it("tracks scale and exposes zoom helpers", () => {
    const zoomIn = vi.fn();
    const zoomOut = vi.fn();
    const resetTransform = vi.fn();

    const { result } = renderHook(() => useZoomableMedia());

    act(() => {
      result.current.ref.current = {
        zoomIn,
        zoomOut,
        resetTransform,
      } as never;
    });

    act(() => {
      result.current.onTransform({} as never, { scale: 2.5 } as never);
    });
    expect(result.current.scale).toBe(2.5);

    act(() => {
      result.current.zoomIn();
      result.current.zoomOut();
      result.current.resetTransform();
    });

    expect(zoomIn).toHaveBeenCalled();
    expect(zoomOut).toHaveBeenCalled();
    expect(resetTransform).toHaveBeenCalled();
  });
});
