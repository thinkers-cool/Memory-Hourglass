import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const {
  enterWindowFullscreen,
  exitWindowFullscreen,
  isWindowFullscreen,
  onResized,
  isTauri,
} = vi.hoisted(() => ({
  enterWindowFullscreen: vi.fn(async () => undefined),
  exitWindowFullscreen: vi.fn(async () => undefined),
  isWindowFullscreen: vi.fn(async () => false),
  onResized: vi.fn(async () => () => undefined),
  isTauri: vi.fn(() => false),
}));

vi.mock("../lib/windowFullscreen", () => ({
  enterWindowFullscreen,
  exitWindowFullscreen,
  isWindowFullscreen,
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onResized,
  }),
}));

import { useSlideshowFullscreen } from "./useSlideshowFullscreen";

describe("useSlideshowFullscreen", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    isTauri.mockReset();
    isTauri.mockReturnValue(false);
    isWindowFullscreen.mockReset();
    isWindowFullscreen.mockResolvedValue(false);
    onResized.mockReset();
    onResized.mockResolvedValue(() => undefined);
    enterWindowFullscreen.mockReset();
    enterWindowFullscreen.mockResolvedValue(undefined);
    exitWindowFullscreen.mockReset();
    exitWindowFullscreen.mockResolvedValue(undefined);
  });

  it("enters fullscreen and listens for browser exit", async () => {
    const onSystemExit = vi.fn();
    const removeListener = vi.spyOn(document, "removeEventListener");
    const { unmount } = renderHook(() => useSlideshowFullscreen(onSystemExit));

    await vi.waitFor(() => {
      expect(enterWindowFullscreen).toHaveBeenCalled();
    });

    document.dispatchEvent(new Event("fullscreenchange"));
    await vi.waitFor(() => {
      expect(onSystemExit).toHaveBeenCalled();
    });

    unmount();
    expect(exitWindowFullscreen).toHaveBeenCalled();
    expect(removeListener).toHaveBeenCalledWith("fullscreenchange", expect.any(Function));
  });

  it("uses tauri resize listener when in desktop app", async () => {
    isTauri.mockReturnValue(true);
    const cleanup = vi.fn();
    let resizeHandler: (() => void) | undefined;
    onResized.mockImplementation(async (handler: () => void) => {
      resizeHandler = handler;
      return cleanup;
    });
    const onSystemExit = vi.fn();

    const { unmount } = renderHook(() => useSlideshowFullscreen(onSystemExit));

    await vi.waitFor(() => {
      expect(onResized).toHaveBeenCalled();
    });

    resizeHandler?.();
    await vi.waitFor(() => {
      expect(onSystemExit).toHaveBeenCalled();
    });

    unmount();
    expect(cleanup).toHaveBeenCalled();
  });

  it("ignores resize before fullscreen is armed", async () => {
    let enterResolve: (() => void) | undefined;
    enterWindowFullscreen.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          enterResolve = resolve;
        }),
    );
    const onSystemExit = vi.fn();
    renderHook(() => useSlideshowFullscreen(onSystemExit));

    document.dispatchEvent(new Event("fullscreenchange"));
    expect(onSystemExit).not.toHaveBeenCalled();

    enterResolve?.();
    await vi.waitFor(() => {
      expect(enterWindowFullscreen).toHaveBeenCalled();
    });
  });

  it("skips setup when unmounted before fullscreen arms", async () => {
    let enterResolve: (() => void) | undefined;
    enterWindowFullscreen.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          enterResolve = resolve;
        }),
    );
    const onSystemExit = vi.fn();
    const { unmount } = renderHook(() => useSlideshowFullscreen(onSystemExit));

    unmount();
    enterResolve?.();
    await new Promise((resolve) => setTimeout(resolve, 0));

    document.dispatchEvent(new Event("fullscreenchange"));
    expect(onSystemExit).not.toHaveBeenCalled();
    expect(exitWindowFullscreen).toHaveBeenCalled();
  });

  it("ignores system exit while still fullscreen", async () => {
    isWindowFullscreen.mockResolvedValue(true);
    const onSystemExit = vi.fn();
    renderHook(() => useSlideshowFullscreen(onSystemExit));

    await vi.waitFor(() => {
      expect(enterWindowFullscreen).toHaveBeenCalled();
    });

    document.dispatchEvent(new Event("fullscreenchange"));
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(onSystemExit).not.toHaveBeenCalled();
  });

  it("ignores duplicate system exit after leaving fullscreen", async () => {
    const onSystemExit = vi.fn();
    renderHook(() => useSlideshowFullscreen(onSystemExit));

    await vi.waitFor(() => {
      expect(enterWindowFullscreen).toHaveBeenCalled();
    });

    document.dispatchEvent(new Event("fullscreenchange"));
    await vi.waitFor(() => {
      expect(onSystemExit).toHaveBeenCalledTimes(1);
    });

    document.dispatchEvent(new Event("fullscreenchange"));
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(onSystemExit).toHaveBeenCalledTimes(1);
  });

  it("ignores tauri resize while still fullscreen", async () => {
    isTauri.mockReturnValue(true);
    isWindowFullscreen.mockResolvedValue(true);
    let resizeHandler: (() => void) | undefined;
    onResized.mockImplementation(async (handler: () => void) => {
      resizeHandler = handler;
      return () => undefined;
    });
    const onSystemExit = vi.fn();
    renderHook(() => useSlideshowFullscreen(onSystemExit));

    await vi.waitFor(() => {
      expect(onResized).toHaveBeenCalled();
    });

    resizeHandler?.();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(onSystemExit).not.toHaveBeenCalled();
  });
});
