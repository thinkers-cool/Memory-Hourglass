import type { AssetFilter } from "../types";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function optionalNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value)
    ? value
    : undefined;
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function optionalStringArray(value: unknown): string[] | undefined {
  if (!Array.isArray(value)) {
    return undefined;
  }
  const items = value.filter(
    (entry): entry is string => typeof entry === "string",
  );
  return items.length > 0 ? items : undefined;
}

function optionalNumberArray(value: unknown): number[] | undefined {
  if (!Array.isArray(value)) {
    return undefined;
  }
  const items = value.filter(
    (entry): entry is number =>
      typeof entry === "number" && Number.isFinite(entry),
  );
  return items.length > 0 ? items : undefined;
}

function optionalBoolean(value: unknown): boolean | undefined {
  return typeof value === "boolean" ? value : undefined;
}

export function parseAssetFilter(value: unknown): AssetFilter | null {
  if (!isRecord(value)) {
    return null;
  }

  const filter: AssetFilter = {};
  const rootId = optionalNumber(value.root_id);
  if (rootId !== undefined) filter.root_id = rootId;
  const ratingMin = optionalNumber(value.rating_min);
  if (ratingMin !== undefined) filter.rating_min = ratingMin;
  const syncStates = optionalStringArray(value.sync_states);
  if (syncStates !== undefined) filter.sync_states = syncStates;
  const kind = optionalString(value.kind);
  if (kind !== undefined) filter.kind = kind;
  const camera = optionalString(value.camera);
  if (camera !== undefined) filter.camera = camera;
  const captureFrom = optionalNumber(value.capture_from);
  if (captureFrom !== undefined) filter.capture_from = captureFrom;
  const captureTo = optionalNumber(value.capture_to);
  if (captureTo !== undefined) filter.capture_to = captureTo;
  const albumIds = optionalNumberArray(value.album_ids);
  if (albumIds !== undefined) filter.album_ids = albumIds;
  const tagIds = optionalNumberArray(value.tag_ids);
  if (tagIds !== undefined) filter.tag_ids = tagIds;
  const metaSearch = optionalString(value.meta_search);
  if (metaSearch !== undefined) filter.meta_search = metaSearch;
  const hasGps = optionalBoolean(value.has_gps);
  if (hasGps !== undefined) filter.has_gps = hasGps;
  const hasDuplicate = optionalBoolean(value.has_duplicate);
  if (hasDuplicate !== undefined) filter.has_duplicate = hasDuplicate;
  const assetIds = optionalNumberArray(value.asset_ids);
  if (assetIds !== undefined) filter.asset_ids = assetIds;
  const deletedOnly = optionalBoolean(value.deleted_only);
  if (deletedOnly !== undefined) filter.deleted_only = deletedOnly;

  return filter;
}

export function parseSmartCollectionFilter(
  filterJson: string,
): AssetFilter | null {
  try {
    const parsed: unknown = JSON.parse(filterJson);
    return parseAssetFilter(parsed);
  } catch {
    return null;
  }
}
