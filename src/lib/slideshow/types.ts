export type SlideshowTheme =
  | "dissolve"
  | "ken-burns"
  | "fade-zoom"
  | "push"
  | "dip-black";

export type SlideshowIntervalMs = 2000 | 3000 | 5000 | 8000;

export interface SlideshowSettings {
  theme: SlideshowTheme;
  intervalMs: SlideshowIntervalMs;
  loop: boolean;
  shuffle: boolean;
  muteVideos: boolean;
}

export type KenBurnsVariant = "zoom-in" | "zoom-out" | "pan-left" | "pan-right";

export interface ThemeConfig {
  transitionMs: number;
}
