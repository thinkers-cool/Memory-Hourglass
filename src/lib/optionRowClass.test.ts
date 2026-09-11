import { describe, expect, it } from "vitest";
import { optionRowClass } from "./optionRowClass";

describe("optionRowClass", () => {
  it("returns active styling when selected", () => {
    expect(optionRowClass(true)).toContain("bg-interactive-selected");
  });

  it("returns hover styling when inactive", () => {
    expect(optionRowClass(false)).toContain("hover:bg-interactive-hover-strong");
  });
});
