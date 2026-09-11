import { describe, expect, it, vi } from "vitest";
import { loadEmojiCatalog, unifiedToEmoji } from "./emojiCatalog";

describe("unifiedToEmoji", () => {
  it("converts single codepoint unified strings", () => {
    expect(unifiedToEmoji("1F600")).toBe("😀");
  });

  it("converts multi-codepoint unified strings", () => {
    expect(unifiedToEmoji("1F468-200D-1F469-200D-1F467")).toBe("👨‍👩‍👧");
  });

  it("skips invalid codepoint segments", () => {
    expect(unifiedToEmoji("ZZZZ-1F600")).toBe("😀");
  });
});

describe("loadEmojiCatalog", () => {
  it("loads and deduplicates emoji entries", async () => {
    const first = await loadEmojiCatalog();
    const second = await loadEmojiCatalog();
    expect(first.length).toBeGreaterThan(0);
    expect(second).toBe(first);
    expect(new Set(first).size).toBe(first.length);
  });

  it("skips duplicate unified entries", async () => {
    vi.resetModules();
    vi.doMock("emoji-picker-react/dist/data/emojis-en.json", () => ({
      default: {
        emojis: {
          smileys: [{ u: "1F600" }, { u: "1F600" }],
        },
      },
    }));
    const { loadEmojiCatalog: loadCatalog } = await import("./emojiCatalog");
    const emojis = await loadCatalog();
    expect(emojis).toEqual(["😀"]);
    vi.doUnmock("emoji-picker-react/dist/data/emojis-en.json");
    vi.resetModules();
  });
});
