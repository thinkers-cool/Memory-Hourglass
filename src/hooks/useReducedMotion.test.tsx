import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useRafTransition } from "./useReducedMotion";

describe("useRafTransition", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("does not jump to 1 when deactivated after a completed run", () => {
    let rafCallback: FrameRequestCallback | null = null;
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
      rafCallback = callback;
      return 1;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => undefined);
    vi.spyOn(performance, "now").mockReturnValue(0);

    const onComplete = vi.fn();
    const { result, rerender } = renderHook(
      ({ active }) => useRafTransition(active, 500, onComplete),
      { initialProps: { active: true } },
    );

    expect(result.current).toBe(0);
    act(() => {
      rafCallback?.(500);
    });
    expect(result.current).toBe(1);
    expect(onComplete).toHaveBeenCalled();

    rerender({ active: false });
    expect(result.current).toBe(1);
  });

  it("resets to 0 synchronously when reactivated", () => {
    vi.spyOn(window, "requestAnimationFrame").mockImplementation(() => 1);
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => undefined);
    vi.spyOn(performance, "now").mockReturnValue(0);

    const { result, rerender } = renderHook(
      ({ active }) => useRafTransition(active, 500, vi.fn()),
      { initialProps: { active: false } },
    );

    expect(result.current).toBe(1);
    rerender({ active: true });
    expect(result.current).toBe(0);
  });
});
