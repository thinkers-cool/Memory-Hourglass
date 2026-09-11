import { describe, expect, it } from "vitest";
import { formatImageViewerZoom } from "./imageViewerZoom";

describe("imageViewerZoom", () => {
  it("formats zoom percentage", () => {
    expect(formatImageViewerZoom(1)).toBe("100%");
    expect(formatImageViewerZoom(1.5)).toBe("150%");
  });
});
