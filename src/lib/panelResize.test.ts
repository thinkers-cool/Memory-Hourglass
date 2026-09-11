import { describe, expect, it } from "vitest";
import { clampLeadingPanelWidth, clampTrailingPanelWidth } from "./panelResize";

describe("clampLeadingPanelWidth", () => {
  it("clamps within container bounds", () => {
    expect(clampLeadingPanelWidth(300, 800, 180, 260)).toBe(300);
    expect(clampLeadingPanelWidth(50, 800, 180, 260)).toBe(180);
    expect(clampLeadingPanelWidth(900, 800, 180, 260)).toBe(532);
  });
});

describe("clampTrailingPanelWidth", () => {
  it("clamps within container bounds", () => {
    expect(clampTrailingPanelWidth(340, 800, 400, 260)).toBe(340);
    expect(clampTrailingPanelWidth(100, 800, 400, 260)).toBe(260);
    expect(clampTrailingPanelWidth(900, 800, 400, 260)).toBe(392);
  });
});
