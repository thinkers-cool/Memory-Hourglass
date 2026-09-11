import { afterEach, describe, expect, it, vi } from "vitest";
import i18n from "../i18n";
import {
  DEFAULT_THEME,
  initTheme,
  isAppTheme,
  readStoredTheme,
  setTheme,
  themeLabel,
  THEME_STORAGE_KEY,
} from "./theme";

afterEach(() => {
  localStorage.clear();
  document.documentElement.setAttribute("data-theme", DEFAULT_THEME);
});

describe("theme", () => {
  it("recognizes app themes", () => {
    expect(isAppTheme("pulse")).toBe(true);
    expect(isAppTheme("atlas")).toBe(true);
    expect(isAppTheme("prism")).toBe(true);
    expect(isAppTheme("dark")).toBe(false);
    expect(isAppTheme("unknown")).toBe(false);
  });

  it("falls back to default theme", () => {
    localStorage.setItem(THEME_STORAGE_KEY, "unknown");
    expect(readStoredTheme()).toBe(DEFAULT_THEME);
  });

  it("falls back when a removed built-in theme is stored", () => {
    localStorage.setItem(THEME_STORAGE_KEY, "nord");
    expect(readStoredTheme()).toBe(DEFAULT_THEME);
  });

  it("persists and applies theme", () => {
    const listener = vi.fn();
    window.addEventListener("memhg-theme-change", listener);
    setTheme("neon");
    expect(document.documentElement.getAttribute("data-theme")).toBe("neon");
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("neon");
    expect(readStoredTheme()).toBe("neon");
    expect(listener).toHaveBeenCalled();
    window.removeEventListener("memhg-theme-change", listener);
  });

  it("returns translated theme labels", () => {
    expect(themeLabel("pulse")).toBe(i18n.t("theme.pulse", { ns: "common" }));
  });

  it("initializes from storage", () => {
    localStorage.setItem(THEME_STORAGE_KEY, "vault");
    expect(initTheme()).toBe("vault");
    expect(document.documentElement.getAttribute("data-theme")).toBe("vault");
  });

  it("falls back to capitalized theme name when label missing", () => {
    const originalExists = i18n.exists.bind(i18n);
    vi.spyOn(i18n, "exists").mockImplementation((key) => {
      if (key === "theme.slate") return false;
      return originalExists(key as never);
    });
    expect(themeLabel("slate")).toBe("Slate");
    vi.restoreAllMocks();
  });

  it("returns default when storage read throws", () => {
    vi.spyOn(window.localStorage, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(readStoredTheme()).toBe(DEFAULT_THEME);
    vi.restoreAllMocks();
  });

  it("ignores storage write failures", () => {
    const listener = vi.fn();
    window.addEventListener("memhg-theme-change", listener);
    vi.spyOn(window.localStorage, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(() => setTheme("neon")).not.toThrow();
    expect(listener).not.toHaveBeenCalled();
    window.removeEventListener("memhg-theme-change", listener);
    vi.restoreAllMocks();
  });
});
