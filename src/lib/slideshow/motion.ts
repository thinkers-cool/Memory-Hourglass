import type { KenBurnsVariant, SlideshowTheme } from "./types";
import { easeInOutCubic, easeInOutSine } from "./timing";

const FADE_ZOOM_DRIFT = 0.04;
const PUSH_SHIFT_PERCENT = 5;

export interface LayerMotion {
  opacity: number;
  transform: string;
}

export function kenBurnsTransform(
  variant: KenBurnsVariant,
  elapsedMs: number,
  dwellMs: number,
): string {
  const t = dwellMs <= 0 ? 1 : Math.min(1, elapsedMs / dwellMs);
  const p = easeInOutSine(t);

  if (variant === "zoom-in") {
    const scale = 1.05 + 0.05 * p;
    const x = -1.5 + 3 * p;
    const y = -1 + 2 * p;
    return `scale(${scale}) translate(${x}%, ${y}%)`;
  }

  if (variant === "zoom-out") {
    const scale = 1.1 - 0.05 * p;
    const x = 1.5 - 3 * p;
    const y = 1 - 2 * p;
    return `scale(${scale}) translate(${x}%, ${y}%)`;
  }

  if (variant === "pan-left") {
    const x = 2 - 4 * p;
    return `scale(1.06) translate(${x}%, 0)`;
  }

  const x = -2 + 4 * p;
  return `scale(1.06) translate(${x}%, 0)`;
}

function crossfadeOpacity(
  theme: SlideshowTheme,
  role: "outgoing" | "incoming",
  crossfade: number,
): number {
  if (theme === "dip-black") {
    if (role === "outgoing") {
      return crossfade <= 0.5 ? 1 - crossfade * 2 : 0;
    }
    return crossfade <= 0.5 ? 0 : (crossfade - 0.5) * 2;
  }

  return role === "outgoing" ? 1 - crossfade : crossfade;
}

function crossfadeTransform(
  theme: SlideshowTheme,
  role: "outgoing" | "incoming",
  crossfade: number,
): string {
  if (theme === "fade-zoom") {
    if (role === "outgoing") {
      return `scale(${1 + FADE_ZOOM_DRIFT * crossfade})`;
    }
    return `scale(${1 - FADE_ZOOM_DRIFT + FADE_ZOOM_DRIFT * crossfade})`;
  }

  if (theme === "push") {
    if (role === "outgoing") {
      return `translateX(${-PUSH_SHIFT_PERCENT * crossfade}%)`;
    }
    return `translateX(${PUSH_SHIFT_PERCENT * (1 - crossfade)}%)`;
  }

  return "none";
}

function combineTransform(primary: string, secondary: string): string {
  if (primary === "none") return secondary;
  if (secondary === "none") return primary;
  return `${primary} ${secondary}`;
}

export function layerMotion(input: {
  theme: SlideshowTheme;
  role: "outgoing" | "incoming";
  crossfadeT: number;
  slideElapsedMs: number;
  dwellMs: number;
  variant: KenBurnsVariant;
}): LayerMotion {
  const crossfade = easeInOutCubic(input.crossfadeT);
  const opacity = crossfadeOpacity(input.theme, input.role, crossfade);
  const transitionTransform = crossfadeTransform(
    input.theme,
    input.role,
    crossfade,
  );
  const dwellTransform =
    input.theme === "ken-burns"
      ? kenBurnsTransform(
          input.variant,
          input.slideElapsedMs,
          input.dwellMs,
        )
      : "none";

  return {
    opacity,
    transform: combineTransform(transitionTransform, dwellTransform),
  };
}
