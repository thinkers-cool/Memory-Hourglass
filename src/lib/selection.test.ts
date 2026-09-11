import { describe, expect, it } from "vitest";
import {
  applyRangeSelection,
  resolveSelectionAnchor,
  resolveSelectionRating,
  selectRange,
  toggleSelection,
} from "./selection";

describe("toggleSelection", () => {
  it("toggles single selection", () => {
    const next = toggleSelection(new Set(), 5, false);
    expect(next.has(5)).toBe(true);
    expect(toggleSelection(next, 5, false).has(5)).toBe(false);
  });

  it("replaces single selection with another item", () => {
    const first = toggleSelection(new Set(), 1, false);
    const second = toggleSelection(first, 2, false);
    expect(second).toEqual(new Set([2]));
  });

  it("supports multi selection add and remove", () => {
    const first = toggleSelection(new Set(), 1, true);
    const second = toggleSelection(first, 2, true);
    expect(second).toEqual(new Set([1, 2]));
    expect(toggleSelection(second, 2, true)).toEqual(new Set([1]));
  });
});

describe("selectRange", () => {
  it("selects contiguous ranges in either direction", () => {
    const items = [{ id: 10 }, { id: 11 }, { id: 12 }];
    expect(selectRange(items, 0, 2)).toEqual([10, 11, 12]);
    expect(selectRange(items, 2, 0)).toEqual([10, 11, 12]);
    expect(selectRange(items, 1, 1)).toEqual([11]);
  });
});

describe("resolveSelectionAnchor", () => {
  const items = [{ id: 10 }, { id: 11 }, { id: 12 }];

  it("prefers last index when valid", () => {
    expect(resolveSelectionAnchor(items, 12, 1)).toBe(1);
  });

  it("falls back to selected id index", () => {
    expect(resolveSelectionAnchor(items, 12, null)).toBe(2);
  });

  it("returns null when no anchor is available", () => {
    expect(resolveSelectionAnchor(items, 99, null)).toBeNull();
    expect(resolveSelectionAnchor(items, null, -1)).toBeNull();
    expect(resolveSelectionAnchor(items, null, 99)).toBeNull();
  });
});

describe("resolveSelectionRating", () => {
  const items = [
    { id: 1, rating: 5 },
    { id: 2, rating: 5 },
    { id: 3, rating: 2 },
    { id: 4, rating: null },
  ];

  it("returns the shared rating for a single or uniform selection", () => {
    expect(resolveSelectionRating(items, new Set([1]))).toBe(5);
    expect(resolveSelectionRating(items, new Set([1, 2]))).toBe(5);
  });

  it("returns null for mixed or unrated selections", () => {
    expect(resolveSelectionRating(items, new Set([1, 3]))).toBeNull();
    expect(resolveSelectionRating(items, new Set([4]))).toBeNull();
    expect(resolveSelectionRating(items, new Set())).toBeNull();
  });

  it("returns null when selected ids are missing from items", () => {
    expect(resolveSelectionRating(items, new Set([99]))).toBeNull();
  });

  it("returns null when shared rating is zero", () => {
    const zeroRated = [{ id: 5, rating: 0 }];
    expect(resolveSelectionRating(zeroRated, new Set([5]))).toBeNull();
  });
});

describe("applyRangeSelection", () => {
  const items = [{ id: 1 }, { id: 2 }, { id: 3 }, { id: 4 }];

  it("replaces selection for shift range", () => {
    expect(applyRangeSelection(items, 0, 2, new Set([99]), false)).toEqual(
      new Set([1, 2, 3]),
    );
  });

  it("merges selection for additive range", () => {
    expect(applyRangeSelection(items, 0, 2, new Set([99]), true)).toEqual(
      new Set([99, 1, 2, 3]),
    );
  });
});
