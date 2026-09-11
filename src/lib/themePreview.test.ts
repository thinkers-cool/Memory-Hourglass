import { describe, expect, it, vi } from "vitest";
import { APP_THEMES } from "./theme";
import {
  pickStrongThemeColor,
  readAllThemePreviewColors,
  readThemePreviewColors,
} from "./themePreview";

describe("themePreview", () => {
  it("picks the more chromatic strong color", () => {
    expect(
      pickStrongThemeColor("oklch(42% 0.04 265)", "oklch(72% 0.12 200)"),
    ).toBe("oklch(72% 0.12 200)");
    expect(
      pickStrongThemeColor("oklch(72% 0.14 185)", "oklch(68% 0.18 320)"),
    ).toBe("oklch(68% 0.18 320)");
  });

  it("falls back when one color is missing", () => {
    expect(pickStrongThemeColor("", "oklch(50% 0.2 285)")).toBe(
      "oklch(50% 0.2 285)",
    );
    expect(pickStrongThemeColor("oklch(42% 0.04 265)", "")).toBe(
      "oklch(42% 0.04 265)",
    );
  });

  it("treats non-oklch colors as zero chroma", () => {
    expect(pickStrongThemeColor("rgb(1, 2, 3)", "oklch(72% 0.12 200)")).toBe(
      "oklch(72% 0.12 200)",
    );
  });

  it("prefers secondary when it is more chromatic", () => {
    expect(
      pickStrongThemeColor("oklch(72% 0.18 320)", "oklch(42% 0.04 265)"),
    ).toBe("oklch(72% 0.18 320)");
  });

  it("handles oklch colors without chroma values", () => {
    expect(
      pickStrongThemeColor("oklch(50% none 90)", "oklch(60% 0.1 180)"),
    ).toBe("oklch(60% 0.1 180)");
  });

  it("reads preview colors from themed probe elements", () => {
    vi.spyOn(window, "getComputedStyle").mockReturnValue({
      getPropertyValue: (name: string) => {
        const values: Record<string, string> = {
          "--mem-surface-panel-bg": "#111111",
          "--color-primary": "#222222",
          "--color-secondary": "oklch(42% 0.04 265)",
          "--color-accent": "oklch(72% 0.12 200)",
        };
        return values[name] ?? "";
      },
    } as CSSStyleDeclaration);

    const colors = readThemePreviewColors("pulse");
    expect(colors.background).toBe("#111111");
    expect(colors.primary).toBe("#222222");
    expect(colors.strong).toBe("oklch(72% 0.12 200)");

    const allColors = readAllThemePreviewColors(APP_THEMES);
    expect(Object.keys(allColors).length).toBe(APP_THEMES.length);

    vi.restoreAllMocks();
  });
});
