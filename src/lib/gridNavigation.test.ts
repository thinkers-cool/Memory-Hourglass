import { describe, expect, it } from "vitest";
import { gridIndex, gridPosition, navigateGridIndex } from "./gridNavigation";

describe("gridIndex", () => {
  it("maps row and column to flat index", () => {
    expect(gridIndex(2, 3, 5)).toBe(13);
  });
});

describe("gridPosition", () => {
  it("maps flat index to row and column", () => {
    expect(gridPosition(13, 5)).toEqual({ row: 2, col: 3 });
  });
});

describe("navigateGridIndex", () => {
  const cols = 4;
  const count = 10;

  it("returns zero for empty grids", () => {
    expect(navigateGridIndex(0, 0, 0, cols, 0)).toBe(0);
    expect(navigateGridIndex(0, 0, 0, 0, 10)).toBe(0);
  });

  it("starts at first item for negative current index", () => {
    expect(navigateGridIndex(-1, 1, 0, cols, count)).toBe(0);
  });

  it("moves down one row", () => {
    expect(navigateGridIndex(1, 0, 1, cols, count)).toBe(5);
  });

  it("moves up one row", () => {
    expect(navigateGridIndex(5, 0, -1, cols, count)).toBe(1);
  });

  it("stays at edge when moving left from first column", () => {
    expect(navigateGridIndex(4, -1, 0, cols, count)).toBe(4);
  });

  it("stays at edge when moving up from first row", () => {
    expect(navigateGridIndex(2, 0, -1, cols, count)).toBe(2);
  });

  it("stays at edge when moving past last row", () => {
    expect(navigateGridIndex(8, 0, 1, cols, count)).toBe(8);
  });

  it("respects short last row", () => {
    expect(navigateGridIndex(9, 1, 0, cols, count)).toBe(9);
  });
});
