import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  clearDecodeCache,
  decodeImage,
  isImageDecoded,
  useDecodedImage,
} from "./imageDecode";

describe("decodeImage", () => {
  afterEach(() => {
    clearDecodeCache();
  });

  it("resolves when image loads", async () => {
    class MockImage {
      onload: (() => void) | null = null;
      set src(_value: string) {
        queueMicrotask(() => this.onload?.());
      }
      decode = vi.fn().mockResolvedValue(undefined);
    }
    vi.stubGlobal("Image", MockImage as unknown as typeof Image);

    await expect(decodeImage("asset:///tmp/1.jpg")).resolves.toBeUndefined();
    expect(isImageDecoded("asset:///tmp/1.jpg")).toBe(true);
    vi.unstubAllGlobals();
  });
});

describe("useDecodedImage", () => {
  afterEach(() => {
    clearDecodeCache();
  });

  it("starts ready when src is already decoded", async () => {
    class MockImage {
      onload: (() => void) | null = null;
      set src(_value: string) {
        queueMicrotask(() => this.onload?.());
      }
      decode = vi.fn().mockResolvedValue(undefined);
    }
    vi.stubGlobal("Image", MockImage as unknown as typeof Image);

    await decodeImage("asset:///tmp/1.jpg");
    const { result } = renderHook(() => useDecodedImage("asset:///tmp/1.jpg"));
    expect(result.current).toBe(true);
    vi.unstubAllGlobals();
  });

  it("becomes ready after decode", async () => {
    class MockImage {
      onload: (() => void) | null = null;
      set src(_value: string) {
        queueMicrotask(() => this.onload?.());
      }
      decode = vi.fn().mockResolvedValue(undefined);
    }
    vi.stubGlobal("Image", MockImage as unknown as typeof Image);

    const { result } = renderHook(() => useDecodedImage("asset:///tmp/1.jpg"));
    expect(result.current).toBe(false);
    await waitFor(() => expect(result.current).toBe(true));
    vi.unstubAllGlobals();
  });

  it("is immediately ready when skipped", () => {
    const { result } = renderHook(() => useDecodedImage(null, true));
    expect(result.current).toBe(true);
  });
});
