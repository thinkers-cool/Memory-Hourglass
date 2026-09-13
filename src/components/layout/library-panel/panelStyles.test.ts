import { describe, expect, it } from "vitest";
import { panelRowButtonClass, rootDisplayName } from "./panelStyles";

describe("rootDisplayName", () => {
  it("returns the last path segment for posix paths", () => {
    expect(rootDisplayName("/Users/tony/Photos")).toBe("Photos");
  });

  it("returns the last path segment for windows paths", () => {
    expect(rootDisplayName("C:\\Users\\tony\\Photos")).toBe("Photos");
  });

  it("strips trailing separators", () => {
    expect(rootDisplayName("C:\\Users\\tony\\Photos\\")).toBe("Photos");
    expect(rootDisplayName("/Volumes/nas/share/")).toBe("share");
  });

  it("falls back to the original path when no segment exists", () => {
    expect(rootDisplayName("/")).toBe("/");
  });
});

describe("panelRowButtonClass", () => {
  it("marks active and dimmed rows", () => {
    expect(panelRowButtonClass(true, true)).toContain("opacity-80");
    expect(panelRowButtonClass(false, false)).not.toContain("opacity-80");
  });
});
