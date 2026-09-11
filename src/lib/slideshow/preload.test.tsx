import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AssetCard } from "../../types";
import { useSlideshowPreload } from "./preload";

function card(id: number, kind: AssetCard["kind"] = "image"): AssetCard {
  return {
    id,
    file_name: `${id}.jpg`,
    ext: "jpg",
    kind,
    capture_at: null,
    rating: null,
    sync_state: "ok",
    thumb_path: `/tmp/${id}.webp`,
    abs_path: `/tmp/${id}.jpg`,
    has_duplicate: false,
  };
}

describe("useSlideshowPreload", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("skips missing and unplayable slides", () => {
    const imageSrcs: string[] = [];
    class MockImage {
      set src(value: string) {
        imageSrcs.push(value);
      }
      decode = vi.fn().mockResolvedValue(undefined);
    }
    vi.stubGlobal("Image", MockImage as unknown as typeof Image);

    renderHook(() =>
      useSlideshowPreload([{ ...card(1), sync_state: "missing" }], 0),
    );
    expect(imageSrcs.length).toBe(0);
  });

  it("preloads neighbors around the current slide", async () => {
    const imageSrcs: string[] = [];
    class MockImage {
      set src(value: string) {
        imageSrcs.push(value);
      }
      decode = vi.fn().mockResolvedValue(undefined);
    }
    vi.stubGlobal("Image", MockImage as unknown as typeof Image);

    const items = [card(1), card(2, "missing"), card(3), card(4)];
    renderHook(({ index }) => useSlideshowPreload(items, index), {
      initialProps: { index: 2 },
    });

    await waitFor(() => expect(imageSrcs.length).toBeGreaterThan(0));
    expect(imageSrcs.some((src) => src.includes("2.webp"))).toBe(true);
    expect(imageSrcs.some((src) => src.includes("3.webp"))).toBe(true);
    vi.unstubAllGlobals();
  });

  it("preloads video metadata for video slides", async () => {
    const videoElements: Array<{
      onloadedmetadata: (() => void) | null;
      onerror: (() => void) | null;
    }> = [];
    const originalCreateElement = document.createElement.bind(document);
    vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
      if (tag === "video") {
        const video = {
          preload: "",
          src: "",
          onloadedmetadata: null as (() => void) | null,
          onerror: null as (() => void) | null,
          removeAttribute: vi.fn(),
          load: vi.fn(),
        };
        videoElements.push(video);
        return video as unknown as HTMLVideoElement;
      }
      return originalCreateElement(tag);
    });

    renderHook(() => useSlideshowPreload([card(1, "video")], 0));

    await waitFor(() => expect(videoElements.length).toBe(1));
    videoElements[0].onloadedmetadata?.();
    videoElements[0].onerror?.();
    vi.restoreAllMocks();
  });

  it("preloads full image when thumb is missing", async () => {
    const imageSrcs: string[] = [];
    class MockImage {
      set src(value: string) {
        imageSrcs.push(value);
      }
      decode = vi.fn().mockRejectedValue(new Error("decode failed"));
    }
    vi.stubGlobal("Image", MockImage as unknown as typeof Image);

    const noThumb = { ...card(1), thumb_path: null };
    renderHook(() => useSlideshowPreload([noThumb], 0));

    await waitFor(() => expect(imageSrcs.some((src) => src.includes("1.jpg"))).toBe(true));
  });

  it("clears video preload after timeout", async () => {
    vi.useFakeTimers();
    const videoElements: Array<{
      removeAttribute: ReturnType<typeof vi.fn>;
      load: ReturnType<typeof vi.fn>;
    }> = [];
    const originalCreateElement = document.createElement.bind(document);
    vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
      if (tag === "video") {
        const video = {
          preload: "",
          src: "",
          onloadedmetadata: null as (() => void) | null,
          onerror: null as (() => void) | null,
          removeAttribute: vi.fn(),
          load: vi.fn(),
        };
        videoElements.push(video);
        return video as unknown as HTMLVideoElement;
      }
      return originalCreateElement(tag);
    });

    renderHook(() => useSlideshowPreload([card(1, "video")], 0));
    expect(videoElements.length).toBe(1);

    vi.advanceTimersByTime(3000);
    expect(videoElements[0].removeAttribute).toHaveBeenCalledWith("src");
    expect(videoElements[0].load).toHaveBeenCalled();
  });
});
