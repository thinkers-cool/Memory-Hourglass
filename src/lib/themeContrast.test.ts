import { describe, expect, it } from "vitest";
import { wcagContrast, oklch } from "culori";
import { APP_THEMES } from "./theme";
import { THEME_CONTRAST_PAIRS } from "./themeColors";

describe("theme contrast", () => {
  for (const theme of APP_THEMES) {
    describe(theme, () => {
      for (const pair of THEME_CONTRAST_PAIRS[theme]) {
        it(`${pair.label} meets WCAG AA`, () => {
          const ratio = wcagContrast(oklch(pair.foreground), oklch(pair.background));
          expect(ratio).toBeGreaterThanOrEqual(pair.minRatio);
        });
      }
    });
  }
});
