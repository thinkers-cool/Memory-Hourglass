import type { SlideshowSettings } from "./types";
import { coerceIntervalMs } from "./timing";
import { loadStoredJson } from "../storage";

const STORAGE_KEY = "memhg.slideshow.settings";

export const DEFAULT_SLIDESHOW_SETTINGS: SlideshowSettings = {
  theme: "dissolve",
  intervalMs: 3000,
  loop: true,
  shuffle: false,
  muteVideos: true,
};

function validateSlideshowSettings(value: unknown): SlideshowSettings | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return null;
  }
  const parsed = value as Partial<SlideshowSettings>;
  const intervalMs =
    parsed.intervalMs !== undefined
      ? coerceIntervalMs(parsed.intervalMs)
      : DEFAULT_SLIDESHOW_SETTINGS.intervalMs;
  return { ...DEFAULT_SLIDESHOW_SETTINGS, ...parsed, intervalMs };
}

export function loadSlideshowSettings(): SlideshowSettings {
  return loadStoredJson(
    STORAGE_KEY,
    DEFAULT_SLIDESHOW_SETTINGS,
    validateSlideshowSettings,
  );
}

export function saveSlideshowSettings(settings: SlideshowSettings): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
}
