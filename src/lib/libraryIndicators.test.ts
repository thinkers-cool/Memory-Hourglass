import { describe, expect, it } from "vitest";
import {
  DEFAULT_ALBUM_EMOJI,
  DEFAULT_TAG_COLOR,
  albumEmoji,
  normalizeTagColor,
  tagColor,
} from "./libraryIndicators";

describe("albumEmoji", () => {
  it("returns custom emoji when present", () => {
    expect(albumEmoji("🌅")).toBe("🌅");
  });

  it("falls back when emoji is empty or missing", () => {
    expect(albumEmoji("")).toBe(DEFAULT_ALBUM_EMOJI);
    expect(albumEmoji(null)).toBe(DEFAULT_ALBUM_EMOJI);
    expect(albumEmoji(undefined)).toBe(DEFAULT_ALBUM_EMOJI);
  });
});

describe("tagColor", () => {
  it("accepts valid hex colors", () => {
    expect(tagColor("#ff0000")).toBe("#ff0000");
    expect(tagColor("#AABBCC")).toBe("#AABBCC");
  });

  it("falls back for invalid or missing colors", () => {
    expect(tagColor("red")).toBe(DEFAULT_TAG_COLOR);
    expect(tagColor("#fff")).toBe(DEFAULT_TAG_COLOR);
    expect(tagColor("")).toBe(DEFAULT_TAG_COLOR);
    expect(tagColor(null)).toBe(DEFAULT_TAG_COLOR);
    expect(tagColor(undefined)).toBe(DEFAULT_TAG_COLOR);
  });
});

describe("normalizeTagColor", () => {
  it("keeps valid hex colors", () => {
    expect(normalizeTagColor("#123456")).toBe("#123456");
  });

  it("replaces invalid values with default", () => {
    expect(normalizeTagColor("invalid")).toBe(DEFAULT_TAG_COLOR);
  });
});
