import type { RawTag } from "../types";

export function filterRawTags(tags: RawTag[], query: string): RawTag[] {
  const trimmed = query.trim().toLowerCase();
  if (!trimmed) return tags;
  return tags.filter(
    (tag) =>
      tag.name.toLowerCase().includes(trimmed) ||
      tag.value.toLowerCase().includes(trimmed),
  );
}
