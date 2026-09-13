import { afterEach, describe, expect, it, vi } from "vitest";
import {
  bindThumbIntersection,
  loadThumbSrc,
  settleThumbLoad,
} from "./gridThumb";
import * as thumbLoad from "./thumbLoad";
import { resetThumbLoadQueueForTests } from "./thumbLoad";

describe("gridThumb", () => {
  afterEach(() => {
    resetThumbLoadQueueForTests();
    vi.restoreAllMocks();
  });

  it("no-ops when the host element is missing", () => {
    const cleanup = bindThumbIntersection(null, vi.fn());
    expect(cleanup).toEqual(expect.any(Function));
    expect(cleanup()).toBeUndefined();
  });

  it("notifies when the host becomes visible", () => {
    let callback: IntersectionObserverCallback | null = null;
    class MockIntersectionObserver {
      constructor(next: IntersectionObserverCallback) {
        callback = next;
      }
      observe() {}
      disconnect() {}
    }
    vi.stubGlobal("IntersectionObserver", MockIntersectionObserver);
    const host = document.createElement("span");
    const onVisible = vi.fn();
    bindThumbIntersection(host, onVisible);
    callback?.(
      [{ isIntersecting: true } as IntersectionObserverEntry],
      {} as IntersectionObserver,
    );
    expect(onVisible).toHaveBeenCalledTimes(1);
    vi.unstubAllGlobals();
  });

  it("ignores non-intersecting entries and disconnects on cleanup", () => {
    let callback: IntersectionObserverCallback | null = null;
    const disconnect = vi.fn();
    class MockIntersectionObserver {
      constructor(next: IntersectionObserverCallback) {
        callback = next;
      }
      observe() {}
      disconnect = disconnect;
    }
    vi.stubGlobal("IntersectionObserver", MockIntersectionObserver);
    const host = document.createElement("span");
    const onVisible = vi.fn();
    const cleanup = bindThumbIntersection(host, onVisible);
    callback?.(
      [{ isIntersecting: false } as IntersectionObserverEntry],
      {} as IntersectionObserver,
    );
    expect(onVisible).not.toHaveBeenCalled();
    cleanup();
    expect(disconnect).toHaveBeenCalled();
    vi.unstubAllGlobals();
  });

  it("settles only once", () => {
    const settled = { current: false };
    const releaseSpy = vi.spyOn(thumbLoad, "releaseThumbLoadSlot");
    settleThumbLoad(settled);
    settleThumbLoad(settled);
    expect(releaseSpy).toHaveBeenCalledTimes(1);
  });

  it("releases the slot when loading is cancelled", async () => {
    let resolveSlot: (() => void) | undefined;
    vi.spyOn(thumbLoad, "acquireThumbLoadSlot").mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          resolveSlot = resolve;
        }),
    );
    const releaseSpy = vi.spyOn(thumbLoad, "releaseThumbLoadSlot");
    let cancelled = false;
    const promise = loadThumbSrc("/tmp/a.webp", (path) => path, () => cancelled);
    cancelled = true;
    resolveSlot?.();
    await expect(promise).resolves.toBeNull();
    expect(releaseSpy).toHaveBeenCalled();
  });
});
