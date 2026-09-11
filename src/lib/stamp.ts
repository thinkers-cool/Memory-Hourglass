import type { AssetCard, AssetDetail } from "../types";

export type StampConfig = {
  rating: number | null;
  tag_ids: number[];
  album_ids: number[];
};

export const EMPTY_STAMP_CONFIG: StampConfig = {
  rating: null,
  tag_ids: [],
  album_ids: [],
};

export type StampAssetSnapshot = {
  rating: number | null;
  tag_ids: number[];
  album_ids: number[];
};

export type StampIndicatorState = "none" | "matched";

export type StampTargetContext = {
  compareOpen: boolean;
  compareItemIds: number[];
  fullView: boolean;
  selectedId: number | null;
  selectedList: number[];
};

export function isStampConfigValid(config: StampConfig): boolean {
  return (
    config.rating !== null ||
    config.tag_ids.length > 0 ||
    config.album_ids.length > 0
  );
}

export function stampReferencesTag(config: StampConfig, tagId: number): boolean {
  return config.tag_ids.includes(tagId);
}

export function stampReferencesAlbum(config: StampConfig, albumId: number): boolean {
  return config.album_ids.includes(albumId);
}

export function assetSnapshotFromDetail(detail: AssetDetail): StampAssetSnapshot {
  return {
    rating: detail.meta?.rating ?? null,
    tag_ids: detail.tag_ids,
    album_ids: detail.album_ids,
  };
}

export function assetSnapshotFromParts(
  rating: number | null,
  tagIds: number[],
  albumIds: number[],
): StampAssetSnapshot {
  return {
    rating,
    tag_ids: tagIds,
    album_ids: albumIds,
  };
}

export function isAssetStamped(
  snapshot: StampAssetSnapshot,
  config: StampConfig,
): boolean {
  if (!isStampConfigValid(config)) {
    return false;
  }
  if (config.rating !== null) {
    if (snapshot.rating !== config.rating) {
      return false;
    }
  }
  for (const tagId of config.tag_ids) {
    if (!snapshot.tag_ids.includes(tagId)) {
      return false;
    }
  }
  for (const albumId of config.album_ids) {
    if (!snapshot.album_ids.includes(albumId)) {
      return false;
    }
  }
  return true;
}

export function resolveStampTargets(ctx: StampTargetContext): number[] {
  if (ctx.compareOpen && ctx.compareItemIds.length >= 2) {
    return [...ctx.compareItemIds];
  }
  if (ctx.fullView && ctx.selectedId !== null) {
    return [ctx.selectedId];
  }
  if (ctx.selectedList.length > 0) {
    return [...ctx.selectedList];
  }
  if (ctx.selectedId !== null) {
    return [ctx.selectedId];
  }
  return [];
}

export function stampIndicatorState(
  card: AssetCard,
  config: StampConfig,
  armed: boolean,
  matchedIds: ReadonlySet<number>,
): StampIndicatorState {
  if (!armed || !isStampConfigValid(config)) {
    return "none";
  }
  if (matchedIds.has(card.id)) {
    return "matched";
  }
  if (
    config.rating !== null &&
    config.tag_ids.length === 0 &&
    config.album_ids.length === 0 &&
    card.rating === config.rating
  ) {
    return "matched";
  }
  return "none";
}

export function pruneStampConfig(
  config: StampConfig,
  tagIds: ReadonlySet<number>,
  albumIds: ReadonlySet<number>,
): StampConfig {
  return {
    rating: config.rating,
    tag_ids: config.tag_ids.filter((id) => tagIds.has(id)),
    album_ids: config.album_ids.filter((id) => albumIds.has(id)),
  };
}
