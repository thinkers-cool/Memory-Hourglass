import { describe, expect, it } from "vitest";
import { sampleCard } from "../test/fixtures";
import { patchAssetThumbs } from "./patchAssetThumbs";

describe("patchAssetThumbs", () => {
  it("updates matching cards in place", () => {
    const items = [
      sampleCard,
      { ...sampleCard, id: 2, file_name: "b.jpg", thumb_path: null },
    ];
    const next = patchAssetThumbs(items, [
      { asset_id: 2, thumb_path: "/tmp/2.webp" },
    ]);
    expect(next[1]?.thumb_path).toBe("/tmp/2.webp");
    expect(next[0]).toBe(items[0]);
  });

  it("returns the same array when thumb updates are empty", () => {
    const items = [sampleCard];
    expect(patchAssetThumbs(items, [])).toBe(items);
  });

  it("returns the same array when nothing changes", () => {
    const items = [sampleCard];
    const next = patchAssetThumbs(items, [
      { asset_id: 99, thumb_path: "/tmp/missing.webp" },
    ]);
    expect(next).toBe(items);
  });
});
