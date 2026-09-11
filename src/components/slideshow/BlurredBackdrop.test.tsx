import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useProgressiveImage } from "./BlurredBackdrop";
import type { AssetCard } from "../../types";

const card: AssetCard = {
  id: 1,
  file_name: "photo.jpg",
  ext: "jpg",
  kind: "image",
  capture_at: null,
  rating: null,
  sync_state: "ok",
  thumb_path: "/tmp/1.webp",
  abs_path: "/tmp/1.jpg",
  has_duplicate: false,
};

describe("useProgressiveImage", () => {
  it("starts with preview source and upgrades to full image", async () => {
    class MockImage {
      onload: (() => void) | null = null;
      set src(_value: string) {
        queueMicrotask(() => this.onload?.());
      }
      decode = vi.fn().mockResolvedValue(undefined);
    }
    vi.stubGlobal("Image", MockImage as unknown as typeof Image);

    const { result } = renderHook(() => useProgressiveImage(card));
    expect(result.current).toBe("asset:///tmp/1.webp");
    await waitFor(() =>
      expect(result.current).toBe("asset:///tmp/1.jpg"),
    );
    vi.unstubAllGlobals();
  });

  it("keeps full source when preview matches full image", () => {
    const { result } = renderHook(() =>
      useProgressiveImage({
        ...card,
        thumb_path: null,
      }),
    );
    expect(result.current).toBe("asset:///tmp/1.jpg");
  });
});
