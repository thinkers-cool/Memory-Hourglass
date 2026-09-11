import type { SlideshowTheme } from "./types";
import { easeInOutCubic } from "./timing";

export interface LayerStyle {
  opacity: number;
  transform: string;
}

export function layerStyle(
  theme: SlideshowTheme,
  role: "outgoing" | "incoming",
  progress: number,
): LayerStyle {
  const t = easeInOutCubic(progress);

  if (theme === "push") {
    const shift = 8;
    if (role === "outgoing") {
      return {
        opacity: 1 - t,
        transform: `translateX(${-shift * t}%)`,
      };
    }
    return {
      opacity: t,
      transform: `translateX(${shift * (1 - t)}%)`,
    };
  }

  if (theme === "fade-zoom") {
    if (role === "outgoing") {
      return {
        opacity: 1 - t,
        transform: `scale(${1 + 0.03 * t})`,
      };
    }
    return {
      opacity: t,
      transform: `scale(${0.97 + 0.03 * t})`,
    };
  }

  if (role === "outgoing") {
    return { opacity: 1 - t, transform: "none" };
  }
  return { opacity: t, transform: "none" };
}
