import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PhotoSlide } from "./PhotoSlide";
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

describe("PhotoSlide", () => {
  it("renders full image after decode", async () => {
    class MockImage {
      onload: (() => void) | null = null;
      set src(_value: string) {
        queueMicrotask(() => this.onload?.());
      }
      decode = vi.fn().mockResolvedValue(undefined);
    }
    vi.stubGlobal("Image", MockImage as unknown as typeof Image);

    render(<PhotoSlide card={card} />);
    const img = screen.getByAltText("photo.jpg");
    expect(img).toHaveAttribute("src", "asset:///tmp/1.jpg");
    expect(img.className).toContain("invisible");
    await waitFor(() => {
      expect(img.className).not.toContain("invisible");
    });
    expect(img.className).toContain("object-cover");
    vi.unstubAllGlobals();
  });
});
