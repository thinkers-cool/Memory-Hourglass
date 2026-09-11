type EmojiEntry = {
  u: string;
};

type EmojiDataset = {
  emojis: Record<string, EmojiEntry[]>;
};

let emojiCatalogCache: string[] | null = null;

export function unifiedToEmoji(unified: string): string {
  return unified
    .split("-")
    .map((part) => parseInt(part, 16))
    .filter((code) => !Number.isNaN(code))
    .map((code) => String.fromCodePoint(code))
    .join("");
}

export async function loadEmojiCatalog(): Promise<string[]> {
  if (emojiCatalogCache) return emojiCatalogCache;

  const { default: data } = (await import(
    "emoji-picker-react/dist/data/emojis-en.json"
  )) as { default: EmojiDataset };

  const emojis: string[] = [];
  const seen = new Set<string>();
  for (const list of Object.values(data.emojis)) {
    for (const entry of list) {
      const emoji = unifiedToEmoji(entry.u);
      if (seen.has(emoji)) continue;
      seen.add(emoji);
      emojis.push(emoji);
    }
  }

  emojiCatalogCache = emojis;
  return emojis;
}
