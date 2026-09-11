import { describe, expect, it } from "vitest";
import { indicesInWindow } from "./preloadWindow";

describe("indicesInWindow", () => {
  it("includes only the current index when window is zero", () => {
    expect(indicesInWindow(2, 5, 0)).toEqual([2]);
  });

  it("includes neighbors within bounds", () => {
    expect(indicesInWindow(2, 5, 1).sort((a, b) => a - b)).toEqual([1, 2, 3]);
  });

  it("clamps at the start of the list", () => {
    expect(indicesInWindow(0, 5, 2).sort((a, b) => a - b)).toEqual([0, 1, 2]);
  });

  it("clamps at the end of the list", () => {
    expect(indicesInWindow(4, 5, 2).sort((a, b) => a - b)).toEqual([2, 3, 4]);
  });
});
