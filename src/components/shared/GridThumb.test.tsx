import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as gridThumb from "../../lib/gridThumb";
import * as thumbLoad from "../../lib/thumbLoad";
import { resetThumbLoadQueueForTests } from "../../lib/thumbLoad";
import { GridThumb } from "./GridThumb";

const { convertFileSrc } = vi.hoisted(() => ({
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
}));

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc,
}));

describe("GridThumb", () => {
  let observerCallback: IntersectionObserverCallback | null = null;

  beforeEach(() => {
    resetThumbLoadQueueForTests();
    vi.restoreAllMocks();
    convertFileSrc.mockImplementation((path: string) => `asset://${path}`);
    class MockIntersectionObserver {
      constructor(callback: IntersectionObserverCallback) {
        observerCallback = callback;
      }
      observe() {}
      disconnect() {}
    }
    vi.stubGlobal("IntersectionObserver", MockIntersectionObserver);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    observerCallback = null;
  });

  it("loads the thumb after intersection and releases the slot on load", async () => {
    const { unmount } = render(
      <GridThumb thumbPath="/tmp/1.webp" alt="photo" className="h-full w-full" />,
    );

    observerCallback?.(
      [{ isIntersecting: true } as IntersectionObserverEntry],
      {} as IntersectionObserver,
    );

    const image = await screen.findByRole("img", { name: "photo" });
    expect(image).toHaveAttribute("src", "asset:///tmp/1.webp");
    image.dispatchEvent(new Event("load"));
    unmount();
  });

  it("ignores null thumb load results", async () => {
    vi.spyOn(gridThumb, "loadThumbSrc").mockResolvedValue(null);
    render(
      <GridThumb thumbPath="/tmp/cancel.webp" alt="cancelled" className="h-full w-full" />,
    );
    observerCallback?.(
      [{ isIntersecting: true } as IntersectionObserverEntry],
      {} as IntersectionObserver,
    );
    await act(async () => {
      await Promise.resolve();
    });
    expect(screen.queryByRole("img", { name: "cancelled" })).toBeNull();
  });

  it("settles on image error", async () => {
    const releaseSpy = vi.spyOn(thumbLoad, "releaseThumbLoadSlot");
    render(
      <GridThumb thumbPath="/tmp/error.webp" alt="broken" className="h-full w-full" />,
    );
    observerCallback?.(
      [{ isIntersecting: true } as IntersectionObserverEntry],
      {} as IntersectionObserver,
    );
    const image = await screen.findByRole("img", { name: "broken" });
    image.dispatchEvent(new Event("error"));
    expect(releaseSpy).toHaveBeenCalled();
  });

  it("settles only once when load events repeat", async () => {
    const releaseSpy = vi.spyOn(thumbLoad, "releaseThumbLoadSlot");
    render(
      <GridThumb thumbPath="/tmp/4.webp" alt="settled" className="h-full w-full" />,
    );

    observerCallback?.(
      [{ isIntersecting: true } as IntersectionObserverEntry],
      {} as IntersectionObserver,
    );

    const image = await screen.findByRole("img", { name: "settled" });
    image.dispatchEvent(new Event("load"));
    image.dispatchEvent(new Event("load"));
    expect(releaseSpy).toHaveBeenCalledTimes(1);
  });
});
