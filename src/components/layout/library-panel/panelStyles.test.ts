import { describe, expect, it } from "vitest";
import { rootDisplayName } from "./panelStyles";

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
});
