import { describe, expect, it } from "vitest";
import { ghostBtnClass, GHOST_BTN_INTERACTIVE } from "./buttonClass";

describe("buttonClass", () => {
  it("builds interactive ghost button classes", () => {
    expect(GHOST_BTN_INTERACTIVE).toBe("btn btn-ghost btn-interactive");
    expect(ghostBtnClass()).toBe(GHOST_BTN_INTERACTIVE);
    expect(ghostBtnClass("btn-sm")).toBe(
      "btn btn-ghost btn-interactive btn-sm",
    );
  });
});
