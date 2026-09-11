import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../i18n";
import { LOCALE_CHANGE_EVENT } from "../i18n/events";
import { useAppLocale } from "./useAppLocale";

describe("useAppLocale", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("exposes locales and changes locale", async () => {
    const { result } = renderHook(() => useAppLocale());
    expect(result.current.locales.length).toBeGreaterThan(0);
    expect(result.current.localeLabel("en-US")).toBeTruthy();

    await act(async () => {
      result.current.setLocale("zh-CN");
    });
    expect(result.current.locale).toBe("zh-CN");
  });

  it("reacts to locale change events", () => {
    const { result } = renderHook(() => useAppLocale());
    act(() => {
      window.dispatchEvent(new Event(LOCALE_CHANGE_EVENT));
    });
    expect(result.current.locale).toBeTruthy();
  });

  it("falls back to stored locale for invalid i18n language", async () => {
    localStorage.setItem("memhg-locale", "zh-CN");
    const originalLanguage = i18n.language;
    await i18n.changeLanguage("fr-FR");
    const { result } = renderHook(() => useAppLocale());
    expect(result.current.locale).toBe("zh-CN");
    await i18n.changeLanguage(originalLanguage);
  });

  it("unsubscribes from locale listeners", () => {
    const removeListener = vi.spyOn(window, "removeEventListener");
    const off = vi.spyOn(i18n, "off");
    const { unmount } = renderHook(() => useAppLocale());
    unmount();
    expect(removeListener).toHaveBeenCalledWith(
      LOCALE_CHANGE_EVENT,
      expect.any(Function),
    );
    expect(off).toHaveBeenCalledWith("languageChanged", expect.any(Function));
    removeListener.mockRestore();
    off.mockRestore();
  });
});
