import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useToolbarCompact } from "./useToolbarCompact";

function mountHook(width: number) {
  const element = document.createElement("div");
  vi.spyOn(element, "getBoundingClientRect").mockReturnValue({
    width,
    height: 40,
    top: 0,
    left: 0,
    right: width,
    bottom: 40,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  });

  const ref = { current: element };
  const { result } = renderHook(() => useToolbarCompact(ref, 720));
  return result;
}

describe("useToolbarCompact", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("stays expanded when width is unknown", async () => {
    const result = mountHook(0);
    await waitFor(() => expect(result.current).toBe(false));
  });

  it("switches to compact below threshold", async () => {
    const result = mountHook(640);
    await waitFor(() => expect(result.current).toBe(true));
  });

  it("stays expanded above threshold", async () => {
    const result = mountHook(900);
    await waitFor(() => expect(result.current).toBe(false));
  });

  it("updates from resize observer callbacks", async () => {
    const callbacks: Array<
      (entries: Array<{ contentRect: { width: number } }>) => void
    > = [];
    class MockResizeObserver {
      constructor(
        private callback: (
          entries: Array<{ contentRect: { width: number } }>,
        ) => void,
      ) {
        callbacks.push(callback);
      }
      observe = vi.fn();
      disconnect = vi.fn();
    }
    vi.stubGlobal("ResizeObserver", MockResizeObserver);

    const element = document.createElement("div");
    vi.spyOn(element, "getBoundingClientRect").mockReturnValue({
      width: 900,
      height: 40,
      top: 0,
      left: 0,
      right: 900,
      bottom: 40,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    });
    const ref = { current: element };
    const { result, unmount } = renderHook(() => useToolbarCompact(ref, 720));

    await waitFor(() => expect(result.current).toBe(false));
    act(() => {
      callbacks[0]?.([]);
      callbacks[0]?.([{ contentRect: { width: 600 } }]);
    });
    expect(result.current).toBe(true);
    unmount();
    vi.unstubAllGlobals();
  });

  it("skips setup when ref is missing", async () => {
    const ref = { current: null };
    const { result } = renderHook(() => useToolbarCompact(ref, 720));
    await waitFor(() => expect(result.current).toBe(false));
  });
});
