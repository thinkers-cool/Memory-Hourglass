import { parse } from "culori";
import type { AppTheme } from "./theme";

export type ThemePreviewColors = {
  background: string;
  primary: string;
  strong: string;
};

export const THEME_PREVIEW_VARS = {
  background: "--mem-surface-panel-bg",
  primary: "--color-primary",
  secondary: "--color-secondary",
  accent: "--color-accent",
} as const;

function oklchChroma(color: string): number {
  const parsed = parse(color);
  if (!parsed || parsed.mode !== "oklch") return 0;
  return parsed.c ?? 0;
}

export function pickStrongThemeColor(
  secondary: string,
  accent: string,
): string {
  if (!secondary) return accent;
  if (!accent) return secondary;
  return oklchChroma(accent) >= oklchChroma(secondary) ? accent : secondary;
}

let probe: HTMLDivElement | null = null;

function getThemeProbe(): HTMLDivElement {
  if (!probe) {
    probe = document.createElement("div");
    probe.hidden = true;
    document.body.appendChild(probe);
  }
  return probe;
}

export function readThemePreviewColors(theme: AppTheme): ThemePreviewColors {
  const el = getThemeProbe();
  el.setAttribute("data-theme", theme);
  const style = getComputedStyle(el);
  const background = style
    .getPropertyValue(THEME_PREVIEW_VARS.background)
    .trim();
  const primary = style.getPropertyValue(THEME_PREVIEW_VARS.primary).trim();
  const secondary = style.getPropertyValue(THEME_PREVIEW_VARS.secondary).trim();
  const accent = style.getPropertyValue(THEME_PREVIEW_VARS.accent).trim();
  return {
    background,
    primary,
    strong: pickStrongThemeColor(secondary, accent),
  };
}

export function readAllThemePreviewColors(
  themes: readonly AppTheme[],
): Record<AppTheme, ThemePreviewColors> {
  return Object.fromEntries(
    themes.map((theme) => [theme, readThemePreviewColors(theme)]),
  ) as Record<AppTheme, ThemePreviewColors>;
}
