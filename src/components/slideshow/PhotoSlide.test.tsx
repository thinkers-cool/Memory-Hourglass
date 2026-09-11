import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
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
  it("renders progressive image", () => {
    render(
      <PhotoSlide
        card={card}
        kenBurns={false}
        kenBurnsVariant="zoom-in"
        dwellMs={3000}
        animate={false}
      />,
    );
    expect(screen.getByAltText("photo.jpg")).toHaveAttribute(
      "src",
      "asset:///tmp/1.webp",
    );
  });

  it.each([
    ["zoom-in", "animate-ken-burns-in"],
    ["zoom-out", "animate-ken-burns-out"],
    ["pan-left", "animate-ken-burns-left"],
    ["pan-right", "animate-ken-burns-right"],
  ] as const)("applies ken burns variant %s", (variant, className) => {
    render(
      <PhotoSlide
        card={card}
        kenBurns
        kenBurnsVariant={variant}
        dwellMs={5000}
        animate
      />,
    );
    const img = screen.getByAltText("photo.jpg");
    expect(img.className).toContain(className);
    expect(img.style.animationDuration).toBe("5000ms");
  });

  it("omits motion class when ken burns disabled", () => {
    render(
      <PhotoSlide
        card={card}
        kenBurns={false}
        kenBurnsVariant="zoom-in"
        dwellMs={3000}
        animate
      />,
    );
    expect(screen.getByAltText("photo.jpg").className).not.toContain("animate-ken-burns");
  });
});
