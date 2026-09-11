import { describe, expect, it } from "vitest";
import { joinFolderPath, relativeFolderPath } from "./smbFolderPath";

describe("smbFolderPath", () => {
  it("returns empty relative path for share root", () => {
    expect(relativeFolderPath("/mnt/share", "/mnt/share")).toBe("");
  });

  it("returns nested relative path", () => {
    expect(relativeFolderPath("/mnt/share", "/mnt/share/Photos/2024")).toBe(
      "Photos/2024",
    );
  });

  it("returns empty relative path when absolute is outside root", () => {
    expect(relativeFolderPath("/mnt/share", "/other/path")).toBe("");
  });

  it("joins root without trailing segments", () => {
    expect(joinFolderPath("/mnt/share/", "")).toBe("/mnt/share");
    expect(joinFolderPath("/mnt/share/", "/")).toBe("/mnt/share");
  });

  it("joins nested relative paths", () => {
    expect(joinFolderPath("/mnt/share", "Photos/2024")).toBe("/mnt/share/Photos/2024");
    expect(joinFolderPath("/mnt/share/", "/Photos/2024/")).toBe("/mnt/share/Photos/2024");
  });
});
