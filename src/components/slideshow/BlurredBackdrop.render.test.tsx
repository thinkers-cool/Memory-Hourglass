import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BlurredBackdrop } from "./BlurredBackdrop";
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

describe("BlurredBackdrop", () => {
  it("renders blurred preview image", () => {
    render(<BlurredBackdrop card={card} />);
    const img = document.querySelector("img");
    expect(img).toHaveAttribute("src", "asset:///tmp/1.webp");
  });

  it("falls back to full path when thumbnail is missing", () => {
    render(
      <BlurredBackdrop
        card={{
          ...card,
          thumb_path: null,
        }}
      />,
    );
    const img = document.querySelector("img");
    expect(img).toHaveAttribute("src", "asset:///tmp/1.jpg");
  });
});
