import { describe, expect, it } from "vitest";
import { layerStyle } from "./transitions";

describe("layerStyle", () => {
  it("crossfades on dissolve", () => {
    expect(layerStyle("dissolve", "outgoing", 0).opacity).toBe(1);
    expect(layerStyle("dissolve", "outgoing", 1).opacity).toBe(0);
    expect(layerStyle("dissolve", "incoming", 1).opacity).toBe(1);
    expect(layerStyle("dissolve", "incoming", 0).transform).toBe("none");
  });

  it("slides on push", () => {
    const outgoing = layerStyle("push", "outgoing", 1);
    const incoming = layerStyle("push", "incoming", 0);
    expect(outgoing.opacity).toBe(0);
    expect(outgoing.transform).toContain("translateX");
    expect(incoming.transform).toContain("translateX");
  });

  it("scales on fade-zoom", () => {
    const outgoing = layerStyle("fade-zoom", "outgoing", 1);
    const incoming = layerStyle("fade-zoom", "incoming", 1);
    expect(outgoing.transform).toContain("scale");
    expect(incoming.transform).toContain("scale");
  });

  it("crossfades on ken-burns without transform", () => {
    expect(layerStyle("ken-burns", "outgoing", 0.5).transform).toBe("none");
    expect(layerStyle("ken-burns", "incoming", 0.5).opacity).toBeGreaterThan(0);
  });
});
