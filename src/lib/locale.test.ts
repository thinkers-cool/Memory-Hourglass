import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  isAppLocale,
  localeLabel,
  readStoredLocale,
  resolveSystemLocale,
  saveStoredLocale,
} from "./locale";

describe("locale", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.stubGlobal("navigator", { language: "en-US" });
  });

  it("detects supported locales", () => {
    expect(isAppLocale("en-US")).toBe(true);
    expect(isAppLocale("zh-CN")).toBe(true);
    expect(isAppLocale("fr-FR")).toBe(false);
  });

  it("maps system language to zh-CN or en-US", () => {
    vi.stubGlobal("navigator", { language: "zh-CN" });
    expect(resolveSystemLocale()).toBe("zh-CN");
    vi.stubGlobal("navigator", { language: "zh-HK" });
    expect(resolveSystemLocale()).toBe("zh-CN");
    vi.stubGlobal("navigator", { language: "en-GB" });
    expect(resolveSystemLocale()).toBe("en-US");
  });

  it("persists locale preference", () => {
    saveStoredLocale("zh-CN");
    expect(readStoredLocale()).toBe("zh-CN");
  });

  it("rejects null and undefined locale values", () => {
    expect(isAppLocale(null)).toBe(false);
    expect(isAppLocale(undefined)).toBe(false);
  });

  it("labels locales", () => {
    expect(localeLabel("en-US")).toBe("English");
    expect(localeLabel("zh-CN")).toBe("简体中文");
  });

  it("falls back when navigator is unavailable", () => {
    vi.stubGlobal("navigator", undefined);
    expect(resolveSystemLocale()).toBe("en-US");
  });

  it("falls back when stored locale is invalid", () => {
    localStorage.setItem("memhg-locale", "fr-FR");
    expect(readStoredLocale()).toBe("en-US");
  });

  it("falls back when localStorage read fails", () => {
    vi.spyOn(window.localStorage, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(readStoredLocale()).toBe("en-US");
    vi.restoreAllMocks();
  });

  it("ignores localStorage write failures", () => {
    vi.spyOn(window.localStorage, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(() => saveStoredLocale("zh-CN")).not.toThrow();
    vi.restoreAllMocks();
  });

  it("sets document language", async () => {
    const { applyDocumentLocale } = await import("./locale");
    applyDocumentLocale("zh-CN");
    expect(document.documentElement.lang).toBe("zh-CN");
  });
});
