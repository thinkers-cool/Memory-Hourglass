import { isTauri } from "@tauri-apps/api/core";
import { homeDir } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { describe, expect, it, vi } from "vitest";
import {
  formatError,
  openFolderDialog,
  pickFolder,
  pickFolders,
} from "./pickFolder";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: vi.fn(() => false),
}));

vi.mock("@tauri-apps/api/path", () => ({
  homeDir: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

describe("formatError", () => {
  it("returns message from Error instances", () => {
    expect(formatError(new Error("something failed"))).toBe("something failed");
  });

  it("stringifies non-error values", () => {
    expect(formatError("plain text")).toBe("plain text");
    expect(formatError(404)).toBe("404");
  });
});

describe("pickFolder", () => {
  it("throws outside the desktop app", async () => {
    await expect(pickFolder()).rejects.toThrow("MemHG desktop app");
    await expect(pickFolders()).rejects.toThrow("MemHG desktop app");
  });

  it("returns null and empty arrays when the dialog is cancelled", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(homeDir).mockResolvedValue("/Users/me");
    vi.mocked(open).mockResolvedValue(null);
    await expect(pickFolder()).resolves.toBeNull();
    await expect(pickFolders()).resolves.toEqual([]);
    vi.mocked(open).mockResolvedValue([]);
    await expect(pickFolder()).resolves.toBeNull();
  });

  it("opens folder picker in tauri", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(homeDir).mockResolvedValue("/Users/me");
    vi.mocked(open).mockResolvedValue("/Users/me/Pictures");
    await expect(
      pickFolder({ title: "Pick", createDirectory: true, multiple: false }),
    ).resolves.toBe("/Users/me/Pictures");
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "Pick",
        directory: true,
        defaultPath: "/Users/me",
        canCreateDirectories: true,
      }),
    );
  });

  it("pickFolders returns multiple paths", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(homeDir).mockResolvedValue("/Users/me");
    vi.mocked(open).mockResolvedValue(["/a", "/b"]);
    await expect(pickFolders()).resolves.toEqual(["/a", "/b"]);
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({ multiple: true }),
    );
  });

  it("returns the first path when dialog returns an array", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(homeDir).mockResolvedValue("/Users/me");
    vi.mocked(open).mockResolvedValue(["/Users/me/Pictures", "/Users/me/Other"]);
    await expect(pickFolder()).resolves.toBe("/Users/me/Pictures");
  });

  it("wraps a single dialog path as an array for pickFolders", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(homeDir).mockResolvedValue("/Users/me");
    vi.mocked(open).mockResolvedValue("/Users/me/Pictures");
    await expect(pickFolders()).resolves.toEqual(["/Users/me/Pictures"]);
  });

  it("defaults dialog options when omitted", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(homeDir).mockResolvedValue("/Users/me");
    vi.mocked(open).mockResolvedValue("/tmp");
    await expect(openFolderDialog({})).resolves.toBe("/tmp");
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({ multiple: false, canCreateDirectories: false }),
    );
  });

  it("opens picker when home directory lookup fails", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(homeDir).mockRejectedValue(new Error("no home"));
    vi.mocked(open).mockResolvedValue("/tmp");
    await expect(pickFolder()).resolves.toBe("/tmp");
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({ defaultPath: undefined }),
    );
  });
});
