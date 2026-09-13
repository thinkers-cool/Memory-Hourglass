import { describe, expect, it } from "vitest";
import {
  INTERVAL_OPTIONS,
  KEN_BURNS_VARIANTS,
  THEME_CONFIG,
  THEME_ORDER,
  VIDEO_END_PADDING_MS,
  coerceIntervalMs,
  coerceTheme,
  easeInOutCubic,
  easeInOutSine,
  kenBurnsVariantForIndex,
  nextInterval,
  nextTheme,
  slideshowThemeLabel,
} from "./timing";

describe("slideshow timing constants", () => {
  it("defines interval and theme metadata", () => {
    expect(INTERVAL_OPTIONS).toEqual([2000, 3000, 5000, 8000]);
    expect(THEME_ORDER).toHaveLength(5);
    expect(slideshowThemeLabel("dissolve")).toBe("Dissolve");
    expect(THEME_CONFIG["ken-burns"].transitionMs).toBe(600);
    expect(VIDEO_END_PADDING_MS).toBe(300);
  });
});

describe("kenBurnsVariantForIndex", () => {
  it("cycles through variants", () => {
    expect(kenBurnsVariantForIndex(0)).toBe(KEN_BURNS_VARIANTS[0]);
    expect(kenBurnsVariantForIndex(KEN_BURNS_VARIANTS.length)).toBe(
      KEN_BURNS_VARIANTS[0],
    );
    expect(kenBurnsVariantForIndex(5)).toBe(KEN_BURNS_VARIANTS[1]);
  });
});

describe("coerceIntervalMs", () => {
  it("returns known intervals unchanged", () => {
    expect(coerceIntervalMs(2000)).toBe(2000);
    expect(coerceIntervalMs(8000)).toBe(8000);
  });

  it("coerces string values", () => {
    expect(coerceIntervalMs("5000")).toBe(5000);
  });

  it("falls back to default for unknown values", () => {
    expect(coerceIntervalMs(9999)).toBe(3000);
    expect(coerceIntervalMs("bad")).toBe(3000);
  });
});

describe("coerceTheme", () => {
  it("accepts known themes", () => {
    expect(coerceTheme("ken-burns")).toBe("ken-burns");
  });

  it("falls back to dissolve for unknown themes", () => {
    expect(coerceTheme("push")).toBe("push");
    expect(coerceTheme("wipe")).toBe("dissolve");
  });
});

describe("nextInterval", () => {
  it("advances through known intervals", () => {
    expect(nextInterval(3000)).toBe(5000);
    expect(nextInterval(8000)).toBe(2000);
  });

  it("starts from first option for unknown values", () => {
    expect(nextInterval(9999)).toBe(2000);
  });
});

describe("nextTheme", () => {
  it("advances through theme order", () => {
    expect(nextTheme("dissolve")).toBe("ken-burns");
    expect(nextTheme("ken-burns")).toBe("fade-zoom");
    expect(nextTheme("fade-zoom")).toBe("push");
    expect(nextTheme("push")).toBe("dip-black");
    expect(nextTheme("dip-black")).toBe("dissolve");
  });

  it("starts from first theme for unknown values", () => {
    expect(nextTheme("unknown" as "dissolve")).toBe("dissolve");
  });
});

describe("easeInOutCubic", () => {
  it("returns 0 at start and 1 at end", () => {
    expect(easeInOutCubic(0)).toBe(0);
    expect(easeInOutCubic(1)).toBe(1);
  });

  it("eases through the midpoint", () => {
    expect(easeInOutCubic(0.5)).toBe(0.5);
    expect(easeInOutCubic(0.25)).toBeGreaterThan(0);
    expect(easeInOutCubic(0.25)).toBeLessThan(0.5);
  });
});

describe("easeInOutSine", () => {
  it("returns 0 at start and 1 at end", () => {
    expect(easeInOutSine(0)).toBeCloseTo(0);
    expect(easeInOutSine(1)).toBe(1);
  });
});
