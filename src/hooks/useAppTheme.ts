import { useCallback, useSyncExternalStore } from "react";
import {
  applyTheme,
  isAppTheme,
  readStoredTheme,
  setTheme as persistTheme,
  type AppTheme,
} from "../lib/theme";

function subscribe(onStoreChange: () => void) {
  const handler = () => onStoreChange();
  window.addEventListener("memhg-theme-change", handler);
  return () => window.removeEventListener("memhg-theme-change", handler);
}

function getThemeSnapshot(): AppTheme {
  const attr = document.documentElement.getAttribute("data-theme");
  if (isAppTheme(attr)) return attr;
  return readStoredTheme();
}

function getServerThemeSnapshot(): AppTheme {
  return readStoredTheme();
}

export function useAppTheme() {
  const theme = useSyncExternalStore(
    subscribe,
    getThemeSnapshot,
    getServerThemeSnapshot,
  );

  const setTheme = useCallback((next: AppTheme) => {
    persistTheme(next);
    window.dispatchEvent(new Event("memhg-theme-change"));
  }, []);

  return { theme, setTheme };
}

export function syncThemeController(theme: AppTheme): void {
  applyTheme(theme);
  const input = document.querySelector<HTMLInputElement>(
    `input.radio[name="memhg-theme"][value="${theme}"]`,
  );
  if (input) input.checked = true;
}
