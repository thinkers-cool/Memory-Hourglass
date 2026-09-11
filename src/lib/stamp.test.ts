import { describe, expect, it } from "vitest";
import {
  EMPTY_STAMP_CONFIG,
  assetSnapshotFromDetail,
  assetSnapshotFromParts,
  isAssetStamped,
  isStampConfigValid,
  pruneStampConfig,
  resolveStampTargets,
  stampIndicatorState,
  stampReferencesAlbum,
  stampReferencesTag,
} from "./stamp";
import { sampleCard, sampleDetail } from "../test/fixtures";

describe("stamp", () => {
  it("validates config requires at least one field", () => {
    expect(isStampConfigValid(EMPTY_STAMP_CONFIG)).toBe(false);
    expect(isStampConfigValid({ ...EMPTY_STAMP_CONFIG, rating: 3 })).toBe(true);
    expect(isStampConfigValid({ ...EMPTY_STAMP_CONFIG, tag_ids: [1] })).toBe(true);
    expect(isStampConfigValid({ ...EMPTY_STAMP_CONFIG, album_ids: [2] })).toBe(true);
  });

  it("detects tag and album references", () => {
    const config = { rating: null, tag_ids: [1, 2], album_ids: [3] };
    expect(stampReferencesTag(config, 1)).toBe(true);
    expect(stampReferencesTag(config, 9)).toBe(false);
    expect(stampReferencesAlbum(config, 3)).toBe(true);
    expect(stampReferencesAlbum(config, 9)).toBe(false);
  });

  it("matches stamped assets across rating tags and albums", () => {
    const config = { rating: 4, tag_ids: [1], album_ids: [2] };
    const snapshot = assetSnapshotFromParts(4, [1, 5], [2]);
    expect(isAssetStamped(snapshot, config)).toBe(true);
    expect(isAssetStamped(assetSnapshotFromParts(3, [1], [2]), config)).toBe(false);
    expect(isAssetStamped(assetSnapshotFromParts(4, [], [2]), config)).toBe(false);
    expect(isAssetStamped(assetSnapshotFromParts(4, [1], []), config)).toBe(false);
  });

  it("resolves targets by view context", () => {
    expect(
      resolveStampTargets({
        compareOpen: true,
        compareItemIds: [10, 20],
        fullView: false,
        selectedId: 1,
        selectedList: [1, 2],
      }),
    ).toEqual([10, 20]);

    expect(
      resolveStampTargets({
        compareOpen: false,
        compareItemIds: [],
        fullView: true,
        selectedId: 5,
        selectedList: [1, 2],
      }),
    ).toEqual([5]);

    expect(
      resolveStampTargets({
        compareOpen: false,
        compareItemIds: [],
        fullView: false,
        selectedId: 5,
        selectedList: [1, 2, 3],
      }),
    ).toEqual([1, 2, 3]);

    expect(
      resolveStampTargets({
        compareOpen: false,
        compareItemIds: [],
        fullView: false,
        selectedId: 5,
        selectedList: [],
      }),
    ).toEqual([5]);

    expect(
      resolveStampTargets({
        compareOpen: false,
        compareItemIds: [],
        fullView: false,
        selectedId: null,
        selectedList: [],
      }),
    ).toEqual([]);
  });

  it("prunes missing tag and album ids from config", () => {
    const pruned = pruneStampConfig(
      { rating: 2, tag_ids: [1, 9], album_ids: [3, 8] },
      new Set([1, 2]),
      new Set([3]),
    );
    expect(pruned).toEqual({ rating: 2, tag_ids: [1], album_ids: [3] });
  });

  it("builds snapshots from asset detail", () => {
    expect(assetSnapshotFromDetail(sampleDetail)).toEqual({
      rating: null,
      tag_ids: sampleDetail.tag_ids,
      album_ids: sampleDetail.album_ids,
    });
    expect(
      assetSnapshotFromDetail({
        ...sampleDetail,
        meta: {
          asset_id: 1,
          rating: 3,
          keywords_json: null,
          camera: null,
          lens: null,
          capture_at: null,
          latitude: null,
          longitude: null,
        },
      }),
    ).toEqual({
      rating: 3,
      tag_ids: sampleDetail.tag_ids,
      album_ids: sampleDetail.album_ids,
    });
  });

  it("treats invalid stamp config as unstamped", () => {
    const snapshot = assetSnapshotFromParts(4, [1], [2]);
    expect(isAssetStamped(snapshot, EMPTY_STAMP_CONFIG)).toBe(false);
  });

  it("derives grid stamp indicator states", () => {
    const config = { rating: 4, tag_ids: [], album_ids: [] };
    expect(
      stampIndicatorState(sampleCard, config, false, new Set()),
    ).toBe("none");
    expect(
      stampIndicatorState(sampleCard, config, true, new Set()),
    ).toBe("none");
    expect(
      stampIndicatorState(
        { ...sampleCard, rating: 4 },
        config,
        true,
        new Set(),
      ),
    ).toBe("matched");
    expect(
      stampIndicatorState(sampleCard, config, true, new Set([sampleCard.id])),
    ).toBe("matched");
  });
});
