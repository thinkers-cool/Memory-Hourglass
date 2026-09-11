import { beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../api/client";
import { runStampToggle } from "./stampMutations";

vi.mock("../api/client", () => ({
  batchUpdateAssetMeta: vi.fn(),
  batchAppendTags: vi.fn(),
  batchRemoveTags: vi.fn(),
  addAlbumItems: vi.fn(),
  removeAlbumItems: vi.fn(),
}));

describe("runStampToggle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("applies stamp when assets are not fully stamped", async () => {
    const config = { rating: 3, tag_ids: [1], album_ids: [2] };
    const result = await runStampToggle([10, 11], config, async () => ({
      rating: null,
      tag_ids: [],
      album_ids: [],
    }));

    expect(result.applied).toBe(2);
    expect(result.unstamped).toBe(0);
    expect(api.batchUpdateAssetMeta).toHaveBeenCalledWith([10, 11], {
      rating: 3,
    });
    expect(api.batchAppendTags).toHaveBeenCalledWith([10, 11], 1);
    expect(api.addAlbumItems).toHaveBeenCalledWith(2, [10, 11]);
  });

  it("returns early for empty asset lists", async () => {
    const config = { rating: 3, tag_ids: [1], album_ids: [2] };
    const result = await runStampToggle([], config, async () => ({
      rating: null,
      tag_ids: [],
      album_ids: [],
    }));

    expect(result).toEqual({
      applied: 0,
      unstamped: 0,
      stampedIds: [],
      unstampedIds: [],
    });
    expect(api.batchUpdateAssetMeta).not.toHaveBeenCalled();
  });

  it("unstamps when all assets already match", async () => {
    const config = { rating: 3, tag_ids: [1], album_ids: [2] };
    const result = await runStampToggle([10], config, async () => ({
      rating: 3,
      tag_ids: [1],
      album_ids: [2],
    }));

    expect(result.applied).toBe(0);
    expect(result.unstamped).toBe(1);
    expect(api.batchUpdateAssetMeta).toHaveBeenCalledWith([10], { rating: 0 });
    expect(api.batchRemoveTags).toHaveBeenCalledWith([10], 1);
    expect(api.removeAlbumItems).toHaveBeenCalledWith(2, [10]);
  });

  it("stamps and unstamps tag-only config without rating updates", async () => {
    const config = { rating: null, tag_ids: [1], album_ids: [2] };
    await runStampToggle([10], config, async () => ({
      rating: null,
      tag_ids: [],
      album_ids: [],
    }));
    expect(api.batchUpdateAssetMeta).not.toHaveBeenCalled();
    expect(api.batchAppendTags).toHaveBeenCalledWith([10], 1);
    expect(api.addAlbumItems).toHaveBeenCalledWith(2, [10]);

    vi.clearAllMocks();
    await runStampToggle([10], config, async () => ({
      rating: null,
      tag_ids: [1],
      album_ids: [2],
    }));
    expect(api.batchUpdateAssetMeta).not.toHaveBeenCalled();
    expect(api.batchRemoveTags).toHaveBeenCalledWith([10], 1);
    expect(api.removeAlbumItems).toHaveBeenCalledWith(2, [10]);
  });
});
