import { describe, expect, it } from "vitest";
import {
  formatRatingMinLabel,
  formatRatingStars,
  isRatingStarFilled,
  ratingStarAriaLabel,
} from "./ratingStars";

describe("ratingStars", () => {
  it("formats star counts", () => {
    expect(formatRatingStars(0)).toBe("");
    expect(formatRatingStars(3)).toBe("★★★");
    expect(formatRatingStars(5)).toBe("★★★★★");
  });

  it("formats minimum rating filter labels", () => {
    expect(formatRatingMinLabel(1)).toBe("★+");
    expect(formatRatingMinLabel(3)).toBe("★★★+");
    expect(formatRatingMinLabel(5)).toBe("★★★★★");
  });

  it("formats aria labels", () => {
    expect(ratingStarAriaLabel(1)).toBe("Rate 1 star");
    expect(ratingStarAriaLabel(4)).toBe("Rate 4 stars");
  });

  it("fills stars up to the selected rating", () => {
    expect(isRatingStarFilled(4, 1)).toBe(true);
    expect(isRatingStarFilled(4, 4)).toBe(true);
    expect(isRatingStarFilled(4, 5)).toBe(false);
    expect(isRatingStarFilled(null, 1)).toBe(false);
  });
});
