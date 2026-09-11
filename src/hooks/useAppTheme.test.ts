import { act, renderHook } from "@testing-library/react";
import { createElement } from "react";
import { renderToString } from "react-dom/server";
import { beforeEach, describe, expect, it } from "vitest";
import { applyTheme, readStoredTheme, setTheme } from "../lib/theme";
import { syncThemeController, useAppTheme } from "./useAppTheme";

describe("useAppTheme", () => {
  beforeEach(() => {
    document.documentElement.removeAttribute("data-theme");
    localStorage.clear();
  });

  it("reads theme from document and updates via setTheme", () => {
    setTheme("vault");
    const { result } = renderHook(() => useAppTheme());
    expect(result.current.theme).toBe("vault");

    act(() => {
      result.current.setTheme("sakura");
    });
    expect(readStoredTheme()).toBe("sakura");
  });

  it("falls back to stored theme for invalid document attribute", () => {
    setTheme("ink");
    document.documentElement.setAttribute("data-theme", "invalid");
    const { result } = renderHook(() => useAppTheme());
    expect(result.current.theme).toBe("ink");
  });

  it("uses stored theme for server snapshot", () => {
    setTheme("ink");
    function ThemeSnapshot() {
      const { theme } = useAppTheme();
      return createElement("span", null, theme);
    }
    expect(renderToString(createElement(ThemeSnapshot))).toContain("ink");
  });

  it("syncs theme controller radio input", () => {
    applyTheme("vault");
    const input = document.createElement("input");
    input.type = "radio";
    input.className = "radio";
    input.name = "memhg-theme";
    input.value = "vault";
    document.body.appendChild(input);
    syncThemeController("vault");
    expect(input).toBeChecked();
    input.remove();
  });
});
