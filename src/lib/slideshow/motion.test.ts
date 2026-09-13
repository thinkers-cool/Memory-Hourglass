import { describe, expect, it } from "vitest";
import { kenBurnsTransform, layerMotion } from "./motion";

describe("layerMotion", () => {
  it("crossfades on dissolve without transform", () => {
    expect(layerMotion({
      theme: "dissolve",
      role: "outgoing",
      crossfadeT: 0,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    }).opacity).toBe(1);
    expect(layerMotion({
      theme: "dissolve",
      role: "outgoing",
      crossfadeT: 1,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    }).opacity).toBe(0);
    expect(layerMotion({
      theme: "dissolve",
      role: "incoming",
      crossfadeT: 1,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    }).transform).toBe("none");
  });

  it("applies ken burns transform during ken-burns theme", () => {
    const motion = layerMotion({
      theme: "ken-burns",
      role: "incoming",
      crossfadeT: 1,
      slideElapsedMs: 1500,
      dwellMs: 3000,
      variant: "zoom-in",
    });
    expect(motion.transform).toContain("scale");
    expect(motion.transform).toContain("translate");
  });

  it("scales during fade-zoom and rests at identity", () => {
    const outgoing = layerMotion({
      theme: "fade-zoom",
      role: "outgoing",
      crossfadeT: 1,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    });
    const incoming = layerMotion({
      theme: "fade-zoom",
      role: "incoming",
      crossfadeT: 1,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    });
    expect(outgoing.transform).toContain("scale");
    expect(incoming.transform).toBe("scale(1)");
  });

  it("shifts horizontally on push", () => {
    const outgoing = layerMotion({
      theme: "push",
      role: "outgoing",
      crossfadeT: 1,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    });
    const incoming = layerMotion({
      theme: "push",
      role: "incoming",
      crossfadeT: 0,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    });
    expect(outgoing.transform).toContain("translateX");
    expect(incoming.transform).toContain("translateX");
  });

  it("dips through black with sequential opacity", () => {
    const outgoingMid = layerMotion({
      theme: "dip-black",
      role: "outgoing",
      crossfadeT: 0.5,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    });
    const incomingMid = layerMotion({
      theme: "dip-black",
      role: "incoming",
      crossfadeT: 0.5,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    });
    expect(outgoingMid.opacity).toBe(0);
    expect(incomingMid.opacity).toBe(0);
    expect(layerMotion({
      theme: "dip-black",
      role: "incoming",
      crossfadeT: 1,
      slideElapsedMs: 0,
      dwellMs: 3000,
      variant: "zoom-in",
    }).opacity).toBe(1);
  });
});

describe("kenBurnsTransform", () => {
  it("starts above 1.0 for zoom-in", () => {
    expect(kenBurnsTransform("zoom-in", 0, 3000)).toContain("scale(1.05");
  });

  it("reaches end state at full dwell", () => {
    expect(kenBurnsTransform("pan-left", 3000, 3000)).toContain("translate(-2%");
  });
});
