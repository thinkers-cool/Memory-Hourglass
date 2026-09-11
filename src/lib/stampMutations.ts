import * as api from "../api/client";
import {
  type StampAssetSnapshot,
  type StampConfig,
  isAssetStamped,
} from "./stamp";

export type StampToggleResult = {
  applied: number;
  unstamped: number;
  stampedIds: number[];
  unstampedIds: number[];
};

export async function runStampToggle(
  assetIds: number[],
  config: StampConfig,
  loadSnapshot: (id: number) => Promise<StampAssetSnapshot>,
): Promise<StampToggleResult> {
  if (assetIds.length === 0) {
    return { applied: 0, unstamped: 0, stampedIds: [], unstampedIds: [] };
  }

  const snapshots = await Promise.all(
    assetIds.map(async (id) => ({
      id,
      snapshot: await loadSnapshot(id),
    })),
  );

  const allStamped = snapshots.every(({ snapshot }) =>
    isAssetStamped(snapshot, config),
  );

  if (allStamped) {
    await unstampAssets(assetIds, config);
    return {
      applied: 0,
      unstamped: assetIds.length,
      stampedIds: [],
      unstampedIds: assetIds,
    };
  }

  await stampAssets(assetIds, config);
  return {
    applied: assetIds.length,
    unstamped: 0,
    stampedIds: assetIds,
    unstampedIds: [],
  };
}

async function stampAssets(
  assetIds: number[],
  config: StampConfig,
): Promise<void> {
  if (config.rating !== null) {
    await api.batchUpdateAssetMeta(assetIds, { rating: config.rating });
  }
  for (const tagId of config.tag_ids) {
    await api.batchAppendTags(assetIds, tagId);
  }
  for (const albumId of config.album_ids) {
    await api.addAlbumItems(albumId, assetIds);
  }
}

async function unstampAssets(
  assetIds: number[],
  config: StampConfig,
): Promise<void> {
  if (config.rating !== null) {
    await api.batchUpdateAssetMeta(assetIds, { rating: 0 });
  }
  for (const tagId of config.tag_ids) {
    await api.batchRemoveTags(assetIds, tagId);
  }
  for (const albumId of config.album_ids) {
    await api.removeAlbumItems(albumId, assetIds);
  }
}
