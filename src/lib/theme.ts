import i18n from "../i18n";
import { DEFAULT_THEME_ID, THEME_IDS, type ThemeId } from "./themes.registry";

export const THEME_STORAGE_KEY = "memhg-theme";

export const APP_THEMES = THEME_IDS;

export type AppTheme = ThemeId;

export const DEFAULT_THEME: AppTheme = DEFAULT_THEME_ID;

export function themeLabel(theme: AppTheme): string {
  const key = `theme.${theme}`;
  if (i18n.exists(key, { ns: "common" })) {
    return i18n.t(key, { ns: "common" });
  }
  return theme.charAt(0).toUpperCase() + theme.slice(1);
}

export function isAppTheme(value: string | null | undefined): value is AppTheme {
  return value !== null && value !== undefined && APP_THEMES.includes(value as AppTheme);
}

export function readStoredTheme(): AppTheme {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (isAppTheme(stored)) return stored;
  } catch {
    return DEFAULT_THEME;
  }
  return DEFAULT_THEME;
}

export function applyTheme(theme: AppTheme): void {
  document.documentElement.setAttribute("data-theme", theme);
}

export function setTheme(theme: AppTheme): void {
  applyTheme(theme);
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    return;
  }
  window.dispatchEvent(new Event("memhg-theme-change"));
}

export function initTheme(): AppTheme {
  const theme = readStoredTheme();
  applyTheme(theme);
  return theme;
}
