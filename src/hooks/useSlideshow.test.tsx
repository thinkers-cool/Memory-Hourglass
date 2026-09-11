import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AssetCard } from "../types";
import { createLocalStorageMock } from "../test/storageMock";
import { useSlideshow } from "./useSlideshow";

vi.mock("../lib/slideshow/preload", () => ({
  useSlideshowPreload: vi.fn(),
}));

function card(id: number, kind: AssetCard["kind"] = "image"): AssetCard {
  return {
    id,
    file_name: `${id}.jpg`,
    ext: "jpg",
    kind,
    capture_at: null,
    rating: null,
    sync_state: "ok",
    thumb_path: null,
    abs_path: `/tmp/${id}.jpg`,
    has_duplicate: false,
  };
}

describe("useSlideshow", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", createLocalStorageMock());
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("does not navigate when only one slide and loop is disabled", () => {
    const onNavigate = vi.fn();
    const { result } = renderHook(() =>
      useSlideshow({
        items: [card(1)],
        index: 0,
        onNavigate,
        reducedMotion: false,
      }),
    );

    act(() => result.current.goNext());
    act(() => result.current.goPrev());
    expect(onNavigate).not.toHaveBeenCalled();
  });

  it("navigates forward and backward", () => {
    const onNavigate = vi.fn();
    const items = [card(1), card(2), card(3)];
    const { result } = renderHook(
      ({ index }) =>
        useSlideshow({
          items,
          index,
          onNavigate,
          reducedMotion: false,
        }),
      { initialProps: { index: 1 } },
    );

    act(() => result.current.goNext());
    expect(onNavigate).toHaveBeenCalledWith(2);

    act(() => result.current.goPrev());
    expect(onNavigate).toHaveBeenCalledWith(0);
  });

  it("cycles settings", () => {
    const { result } = renderHook(() =>
      useSlideshow({
        items: [card(1), card(2)],
        index: 0,
        onNavigate: vi.fn(),
        reducedMotion: false,
      }),
    );

    act(() => result.current.cycleInterval());
    act(() => result.current.cycleTheme());
    act(() => result.current.toggleLoop());
    act(() => result.current.toggleMute());
    act(() => result.current.toggleShuffle());

    expect(result.current.settings.intervalMs).toBe(5000);
    expect(result.current.settings.theme).toBe("ken-burns");
    expect(result.current.settings.loop).toBe(false);
    expect(result.current.settings.muteVideos).toBe(false);
    expect(result.current.settings.shuffle).toBe(true);
  });

  it("uses dissolve theme when reduced motion is enabled", () => {
    const { result } = renderHook(() =>
      useSlideshow({
        items: [card(1)],
        index: 0,
        onNavigate: vi.fn(),
        reducedMotion: true,
      }),
    );
    expect(result.current.theme).toBe("dissolve");
    expect(result.current.transitionMs).toBe(0);
    expect(result.current.kenBurns).toBe(false);
  });

  it("advances after video ends when playing", () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const onNavigate = vi.fn();
    const { result } = renderHook(() =>
      useSlideshow({
        items: [card(1, "video"), card(2)],
        index: 0,
        onNavigate,
        reducedMotion: true,
      }),
    );

    act(() => result.current.setPlaying(true));
    act(() => result.current.onVideoEnded());
    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(onNavigate).toHaveBeenCalledWith(1);
    vi.useRealTimers();
  });

  it("does not advance video when paused", () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const onNavigate = vi.fn();
    const { result } = renderHook(() =>
      useSlideshow({
        items: [card(1, "video"), card(2)],
        index: 0,
        onNavigate,
        reducedMotion: true,
      }),
    );

    act(() => result.current.setPlaying(false));
    act(() => result.current.onVideoEnded());
    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(onNavigate).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it("auto-advances image slides on interval", () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const onNavigate = vi.fn();
    renderHook(() =>
      useSlideshow({
        items: [card(1), card(2)],
        index: 0,
        onNavigate,
        reducedMotion: true,
      }),
    );

    act(() => {
      vi.advanceTimersByTime(3000);
    });
    expect(onNavigate).toHaveBeenCalledWith(1);
  });

  it("releases wake lock on unmount", async () => {
    const release = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", {
      wakeLock: {
        request: vi.fn().mockResolvedValue({ release }),
      },
    });

    const { unmount } = renderHook(() =>
      useSlideshow({
        items: [card(1), card(2)],
        index: 0,
        onNavigate: vi.fn(),
        reducedMotion: true,
      }),
    );

    await vi.waitFor(() => {
      expect(navigator.wakeLock?.request).toHaveBeenCalled();
    });
    unmount();
    expect(release).toHaveBeenCalled();
    vi.unstubAllGlobals();
  });

  it("ignores wake lock failures", async () => {
    vi.stubGlobal("navigator", {
      wakeLock: {
        request: vi.fn().mockRejectedValue(new Error("denied")),
      },
    });

    const { unmount } = renderHook(() =>
      useSlideshow({
        items: [card(1), card(2)],
        index: 0,
        onNavigate: vi.fn(),
        reducedMotion: true,
      }),
    );

    await vi.waitFor(() => {
      expect(navigator.wakeLock?.request).toHaveBeenCalled();
    });
    unmount();
    vi.unstubAllGlobals();
  });

  it("clears shuffle order when toggled off", () => {
    const { result } = renderHook(() =>
      useSlideshow({
        items: [card(1), card(2), card(3)],
        index: 0,
        onNavigate: vi.fn(),
        reducedMotion: false,
      }),
    );

    act(() => result.current.toggleShuffle());
    act(() => result.current.toggleShuffle());
    expect(result.current.settings.shuffle).toBe(false);
  });

  it("clears video timers when ending repeatedly", () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const onNavigate = vi.fn();
    const { result } = renderHook(() =>
      useSlideshow({
        items: [card(1, "video"), card(2)],
        index: 0,
        onNavigate,
        reducedMotion: true,
      }),
    );

    act(() => {
      result.current.onVideoEnded();
      result.current.onVideoEnded();
      vi.advanceTimersByTime(300);
    });
    expect(onNavigate).toHaveBeenCalledWith(1);
  });

  it("completes slide transitions", () => {
    let rafCallback: FrameRequestCallback | null = null;
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
      rafCallback = callback;
      return 1;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(
      () => undefined,
    );
    vi.spyOn(performance, "now").mockReturnValue(0);

    const { result, rerender } = renderHook(
      ({ index }) =>
        useSlideshow({
          items: [card(1), card(2)],
          index,
          onNavigate: vi.fn(),
          reducedMotion: false,
        }),
      { initialProps: { index: 0 } },
    );

    rerender({ index: 1 });
    act(() => {
      rafCallback?.(5000);
    });
    expect(result.current.fromIndex).toBeNull();
    vi.restoreAllMocks();
  });

  it("rebuilds shuffle order when item count changes", () => {
    const onNavigate = vi.fn();
    const { result, rerender } = renderHook(
      ({ items, index }) =>
        useSlideshow({
          items,
          index,
          onNavigate,
          reducedMotion: false,
        }),
      {
        initialProps: { items: [card(1), card(2), card(3)], index: 0 },
      },
    );

    act(() => result.current.toggleShuffle());
    rerender({ items: [card(1), card(2), card(3), card(4)], index: 0 });
    act(() => result.current.goNext());
    expect(onNavigate.mock.calls.length).toBeGreaterThan(0);
  });

  it("exposes transition state when index changes", () => {
    const { result, rerender } = renderHook(
      ({ index }) =>
        useSlideshow({
          items: [card(1), card(2)],
          index,
          onNavigate: vi.fn(),
          reducedMotion: false,
        }),
      { initialProps: { index: 0 } },
    );

    rerender({ index: 1 });
    expect(result.current.fromIndex).toBe(0);
    expect(result.current.toIndex).toBe(1);
  });
});
