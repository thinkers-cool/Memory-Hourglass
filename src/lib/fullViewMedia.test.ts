import { afterEach, describe, expect, it, vi } from "vitest";
import { sampleCard, sampleVideoCard } from "../test/fixtures";
import { prefetchFullViewNeighbors, preloadFullViewImage } from "./fullViewMedia";

describe("preloadFullViewImage", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("resolves immediately when the image is already cached", async () => {
    class CachedImage {
      complete = true;
      onload: (() => void) | null = null;
      set src(_value: string) {}
    }
    vi.stubGlobal("Image", CachedImage);

    await expect(preloadFullViewImage("/tmp/1.jpg")).resolves.toBeUndefined();
  });

  it("waits for onload when the image is not cached", async () => {
    class LoadingImage {
      complete = false;
      onload: (() => void) | null = null;
      set src(_value: string) {
        queueMicrotask(() => {
          this.onload?.();
        });
      }
    }
    vi.stubGlobal("Image", LoadingImage);

    await expect(preloadFullViewImage("/tmp/1.jpg")).resolves.toBeUndefined();
  });
});

describe("prefetchFullViewNeighbors", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("prefetches image neighbors only", () => {
    const ImageMock = vi.fn(function ImageMock(this: { src: string }) {
      this.src = "";
    });
    vi.stubGlobal("Image", ImageMock);

    prefetchFullViewNeighbors([sampleCard, sampleVideoCard]);

    expect(ImageMock).toHaveBeenCalledTimes(1);
  });
});
