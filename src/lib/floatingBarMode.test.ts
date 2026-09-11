import { describe, expect, it } from "vitest";
import {
  resolveFloatingBarMode,
  shouldShowFloatingBar,
} from "./floatingBarMode";

describe("resolveFloatingBarMode", () => {
  it("uses trash mode when viewing deleted items", () => {
    expect(resolveFloatingBarMode("deleted")).toBe("trash");
  });

  it("uses library mode otherwise", () => {
    expect(resolveFloatingBarMode("")).toBe("library");
  });
});

describe("shouldShowFloatingBar", () => {
  it("hides when nothing is selected", () => {
    expect(shouldShowFloatingBar(0, false, false)).toBe(false);
  });

  it("hides when gallery or compare is open", () => {
    expect(shouldShowFloatingBar(2, true, false)).toBe(false);
    expect(shouldShowFloatingBar(2, false, true)).toBe(false);
  });

  it("shows when items are selected and overlays are closed", () => {
    expect(shouldShowFloatingBar(3, false, false)).toBe(true);
  });
});
