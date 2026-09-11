import i18n from "../../i18n";
import type {
  KenBurnsVariant,
  SlideshowIntervalMs,
  SlideshowTheme,
  ThemeConfig,
} from "./types";

export const INTERVAL_OPTIONS: readonly SlideshowIntervalMs[] = [
  2000, 3000, 5000, 8000,
];

export const THEME_ORDER: readonly SlideshowTheme[] = [
  "dissolve",
  "ken-burns",
  "push",
  "fade-zoom",
];

export function slideshowThemeLabel(theme: SlideshowTheme): string {
  const keyMap: Record<SlideshowTheme, string> = {
    dissolve: "library:slideshow.effectDissolve",
    "ken-burns": "library:slideshow.effectKenBurns",
    push: "library:slideshow.effectPush",
    "fade-zoom": "library:slideshow.effectFadeZoom",
  };
  return i18n.t(keyMap[theme]);
}

export const THEME_CONFIG: Record<SlideshowTheme, ThemeConfig> = {
  dissolve: {
    transitionMs: 500,
    easing: "cubic-bezier(0.4, 0, 0.2, 1)",
    kenBurns: false,
  },
  "ken-burns": {
    transitionMs: 600,
    easing: "cubic-bezier(0.4, 0, 0.2, 1)",
    kenBurns: true,
  },
  push: {
    transitionMs: 400,
    easing: "cubic-bezier(0.25, 0.1, 0.25, 1)",
    kenBurns: false,
  },
  "fade-zoom": {
    transitionMs: 450,
    easing: "cubic-bezier(0, 0, 0.2, 1)",
    kenBurns: false,
  },
};

export const VIDEO_END_PADDING_MS = 300;

export const KEN_BURNS_VARIANTS: readonly KenBurnsVariant[] = [
  "zoom-in",
  "zoom-out",
  "pan-left",
  "pan-right",
];

export function kenBurnsVariantForIndex(index: number): KenBurnsVariant {
  return KEN_BURNS_VARIANTS[index % KEN_BURNS_VARIANTS.length];
}

export function coerceIntervalMs(value: unknown): SlideshowIntervalMs {
  const ms = typeof value === "string" ? Number(value) : value;
  if (
    typeof ms === "number" &&
    INTERVAL_OPTIONS.includes(ms as SlideshowIntervalMs)
  ) {
    return ms as SlideshowIntervalMs;
  }
  return 3000;
}

export function nextInterval(current: number): SlideshowIntervalMs {
  const idx = INTERVAL_OPTIONS.indexOf(current as SlideshowIntervalMs);
  const next = idx < 0 ? 0 : (idx + 1) % INTERVAL_OPTIONS.length;
  return INTERVAL_OPTIONS[next];
}

export function nextTheme(current: SlideshowTheme): SlideshowTheme {
  const idx = THEME_ORDER.indexOf(current);
  const next = idx < 0 ? 0 : (idx + 1) % THEME_ORDER.length;
  return THEME_ORDER[next];
}

export function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}
