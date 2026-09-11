import { describe, expect, it } from "vitest";
import {
  parseAssetFilter,
  parseSmartCollectionFilter,
} from "./assetFilterValidation";

describe("parseAssetFilter", () => {
  it("returns null for non-object values", () => {
    expect(parseAssetFilter(null)).toBeNull();
    expect(parseAssetFilter("x")).toBeNull();
    expect(parseAssetFilter([])).toBeNull();
  });

  it("returns empty filter for empty object", () => {
    expect(parseAssetFilter({})).toEqual({});
  });

  it("parses all supported fields", () => {
    expect(
      parseAssetFilter({
        root_id: 1,
        rating_min: 3,
        sync_states: ["ok", "missing"],
        kind: "image",
        camera: "Sony",
        capture_from: 100,
        capture_to: 200,
        album_ids: [1, 2],
        tag_ids: [3],
        meta_search: "sunset",
        has_gps: true,
        has_duplicate: false,
        asset_ids: [9],
        deleted_only: true,
      }),
    ).toEqual({
      root_id: 1,
      rating_min: 3,
      sync_states: ["ok", "missing"],
      kind: "image",
      camera: "Sony",
      capture_from: 100,
      capture_to: 200,
      album_ids: [1, 2],
      tag_ids: [3],
      meta_search: "sunset",
      has_gps: true,
      has_duplicate: false,
      asset_ids: [9],
      deleted_only: true,
    });
  });

  it("skips invalid optional values", () => {
    expect(
      parseAssetFilter({
        root_id: "bad",
        rating_min: NaN,
        sync_states: [1, 2],
        kind: "",
        camera: "",
        album_ids: ["x"],
        tag_ids: [],
        meta_search: "",
        has_gps: "yes",
        asset_ids: [NaN],
      }),
    ).toEqual({});
  });
});

describe("parseSmartCollectionFilter", () => {
  it("parses valid json", () => {
    expect(parseSmartCollectionFilter('{"tag_ids":[1]}')).toEqual({
      tag_ids: [1],
    });
  });

  it("returns null for invalid json", () => {
    expect(parseSmartCollectionFilter("{")).toBeNull();
  });
});
