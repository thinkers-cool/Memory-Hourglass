import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useReducedMotion, useRafTransition } from "./useReducedMotion";

describe("useReducedMotion", () => {
  it("reads prefers-reduced-motion on mount", () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: query.includes("reduce"),
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(true);
  });
});

describe("useRafTransition", () => {
  it("returns 1 immediately when inactive", () => {
    const onComplete = vi.fn();
    const { result } = renderHook(() =>
      useRafTransition(false, 300, onComplete),
    );
    expect(result.current).toBe(1);
    expect(onComplete).not.toHaveBeenCalled();
  });

  it("completes immediately when duration is zero", () => {
    const onComplete = vi.fn();
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb) => {
      cb(16);
      return 1;
    });

    renderHook(() => useRafTransition(true, 0, onComplete));
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  it("animates toward completion when active", async () => {
    const onComplete = vi.fn();
    let frame = 0;
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb) => {
      frame += 1;
      cb(frame * 50);
      return frame;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(
      () => undefined,
    );
    vi.spyOn(performance, "now").mockReturnValue(0);

    const { result } = renderHook(() =>
      useRafTransition(true, 100, onComplete),
    );
    await waitFor(() => expect(onComplete).toHaveBeenCalled());
    expect(result.current).toBe(1);
  });
});
