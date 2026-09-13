import { describe, expect, it } from "vitest";
import {
  imageRotationStyle,
  normalizeRotation,
  rotateClockwise,
  rotateCounterClockwise,
} from "./imageRotation";

describe("imageRotation", () => {
  it("normalizes degrees to 0-359", () => {
    expect(normalizeRotation(450)).toBe(90);
    expect(normalizeRotation(-90)).toBe(270);
  });

  it("rotates clockwise and counter-clockwise", () => {
    expect(rotateClockwise(0)).toBe(90);
    expect(rotateCounterClockwise(90)).toBe(0);
    expect(rotateClockwise(270)).toBe(0);
  });

  it("returns transform style for non-zero rotation", () => {
    expect(imageRotationStyle(0)).toEqual({});
    expect(imageRotationStyle(90)).toEqual({ transform: "rotate(90deg)" });
  });
});
