export const DEFAULT_ALBUM_EMOJI = "📁";
export const DEFAULT_TAG_COLOR = "#6b7280";

const HEX_COLOR_PATTERN = /^#[0-9A-Fa-f]{6}$/;

export function albumEmoji(emoji: string | null | undefined): string {
  return emoji && emoji.length > 0 ? emoji : DEFAULT_ALBUM_EMOJI;
}

export function resolveTagColor(
  color: string | null | undefined,
  fallback = DEFAULT_TAG_COLOR,
): string {
  if (color && HEX_COLOR_PATTERN.test(color)) {
    return color;
  }
  return fallback;
}

export const tagColor = resolveTagColor;
export const normalizeTagColor = (color: string) => resolveTagColor(color);
