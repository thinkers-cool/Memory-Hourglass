import { describe, expect, it } from "vitest";
import { listRowClass, menuPickerItemClass, optionRowClass } from "./interactionClass";

describe("interactionClass", () => {
  it("builds list row classes", () => {
    expect(listRowClass(true)).toContain("bg-interactive-selected");
    expect(listRowClass(false)).toContain("hover:bg-interactive-hover");
  });

  it("builds option row classes", () => {
    expect(optionRowClass(true)).toContain("text-primary");
    expect(optionRowClass(false)).toContain("hover:bg-interactive-hover-strong");
  });

  it("builds menu picker item classes", () => {
    expect(menuPickerItemClass(true)).toContain("menu-picker-item-active");
    expect(menuPickerItemClass(false)).toContain("hover:bg-interactive-hover-strong");
  });
});
