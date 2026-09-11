import { describe, expect, it } from "vitest";
import {
  INPUT_CONTROL_CLASS,
  MENU_PICKER_LIST_CLASS,
  MENU_POPOVER_CLASS,
  RANGE_CONTROL_CLASS,
  SELECT_CONTROL_CLASS,
} from "./formControlClass";

describe("formControlClass", () => {
  it("uses daisyUI control primitives", () => {
    expect(INPUT_CONTROL_CLASS).toContain("input input-bordered");
    expect(SELECT_CONTROL_CLASS).toContain("select select-sm select-bordered");
    expect(RANGE_CONTROL_CLASS).toContain("range range-primary");
    expect(MENU_POPOVER_CLASS).toContain("menu menu-sm");
    expect(MENU_PICKER_LIST_CLASS).toContain("menu-picker");
  });
});
