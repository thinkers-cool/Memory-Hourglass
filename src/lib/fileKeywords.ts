export function parseKeywordsJson(json: string | null | undefined): string[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((entry): entry is string => typeof entry === "string");
  } catch {
    return [];
  }
}

export function fileKeywordsNotInCatalog(
  keywordsJson: string | null | undefined,
  catalogTagNames: string[],
): string[] {
  const catalog = new Set(catalogTagNames);
  return parseKeywordsJson(keywordsJson).filter((keyword) => !catalog.has(keyword));
}
