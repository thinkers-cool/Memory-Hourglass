export type SlideshowTheme = "dissolve" | "ken-burns" | "push" | "fade-zoom";

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
  easing: string;
  kenBurns: boolean;
}

export interface SlideTransition {
  from: number;
  to: number;
  progress: number;
}
