import { describe, expect, it } from "vitest";
import { formatSmbAddress, parseSmbAddress, parseSmbHost } from "./smbAddress";

describe("smbAddress", () => {
  it("parses smb host only", () => {
    expect(parseSmbHost("192.168.1.10")).toBe("192.168.1.10");
    expect(parseSmbHost("smb://nas.local")).toBe("nas.local");
    expect(parseSmbHost("smb://nas.local/photos")).toBe("nas.local");
    expect(parseSmbHost("")).toBeNull();
    expect(parseSmbHost("bad host")).toBeNull();
  });

  it("parses smb urls", () => {
    expect(parseSmbAddress("smb://nas.local/photos")).toEqual({
      host: "nas.local",
      share: "photos",
    });
    expect(parseSmbAddress("192.168.1.10/media")).toEqual({
      host: "192.168.1.10",
      share: "media",
    });
  });

  it("rejects invalid addresses", () => {
    expect(parseSmbAddress("nas.local")).toBeNull();
    expect(parseSmbAddress("smb://nas.local/")).toBeNull();
    expect(parseSmbAddress("")).toBeNull();
  });

  it("formats smb urls", () => {
    expect(formatSmbAddress("nas.local", "photos")).toBe(
      "smb://nas.local/photos",
    );
  });
});
