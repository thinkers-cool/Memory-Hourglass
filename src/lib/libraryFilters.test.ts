import { describe, expect, it } from "vitest";
import type { FilterBarState } from "./libraryActions";
import type { Album, TagDto } from "../types";
import {
  FILTER_DISPLAY_ORDER,
  FILTER_IDS,
  applyLibraryFilterChange,
  assetFilterScope,
  buildFilterFromBar,
  buildLibraryFilterDefs,
  clearLibraryFilters,
  dateInputEndToUnix,
  dateInputToUnix,
  emptyFilterBarState,
  filterBarFromAssetFilter,
  filterValuesFromBar,
  isAlbumSourceActive,
  isRootSourceActive,
  isTagSourceActive,
  isValidCaptureUnix,
  mergeLibraryFilter,
  multiFilterValuesFromBar,
  removeLibraryFilter,
  unixToDateInput,
} from "./libraryFilters";

const baseBar: FilterBarState = {
  ratingMin: "",
  syncStates: [],
  deleteStatus: "",
  camera: "",
  tagIds: [],
  albumIds: [],
  metaSearch: "",
  hasGps: false,
  hasDuplicate: false,
  captureFrom: "",
  captureTo: "",
  sort: "date",
  sortDir: "desc",
};

function applyChange(
  bar: FilterBarState,
  id: string,
  value: string,
): FilterBarState {
  let next = bar;
  applyLibraryFilterChange(id, value, (update) => {
    next = typeof update === "function" ? update(bar) : update;
  });
  return next;
}

function removeFilter(bar: FilterBarState, id: string): FilterBarState {
  let next = bar;
  removeLibraryFilter(id, (update) => {
    next = typeof update === "function" ? update(bar) : update;
  });
  return next;
}

describe("date helpers", () => {
  it("converts date inputs to unix timestamps", () => {
    expect(dateInputToUnix("")).toBeUndefined();
    expect(dateInputToUnix("invalid")).toBeUndefined();
    expect(dateInputEndToUnix("invalid")).toBeUndefined();
    expect(dateInputToUnix("2024-06-15")).toBe(
      Math.floor(Date.parse("2024-06-15T00:00:00") / 1000),
    );
    expect(dateInputEndToUnix("2024-06-15")).toBe(
      Math.floor(Date.parse("2024-06-15T23:59:59") / 1000),
    );
  });

  it("roundtrips unix timestamps through date inputs", () => {
    const unix = 1_700_000_000;
    const input = unixToDateInput(unix);
    expect(input).not.toBe("");
    expect(unixToDateInput(undefined)).toBe("");
    expect(unixToDateInput(null)).toBe("");
    expect(unixToDateInput(0)).toBe("");
  });

  it("treats invalid capture dates as unset in filter bar", () => {
    expect(
      filterBarFromAssetFilter(
        {
          album_ids: [3],
          capture_from: null as unknown as number,
          capture_to: null as unknown as number,
        },
        "date",
        "desc",
      ).captureFrom,
    ).toBe("");
    expect(isValidCaptureUnix(1_700_000_000)).toBe(true);
    expect(isValidCaptureUnix(0)).toBe(false);
    expect(isValidCaptureUnix(-1)).toBe(false);
    expect(isValidCaptureUnix(Number.NaN)).toBe(false);
    expect(isValidCaptureUnix("1700000000")).toBe(false);
  });
});

describe("libraryFilters", () => {
  it("builds album filter from toolbar state", () => {
    const filter = buildFilterFromBar({ ...baseBar, albumIds: [7] });
    expect(filter.album_ids).toEqual([7]);
  });

  it("builds multi album and tag filters", () => {
    const filter = buildFilterFromBar({
      ...baseBar,
      albumIds: [1, 2],
      tagIds: [1, 2],
      syncStates: ["ok", "new"],
    });
    expect(filter.album_ids).toEqual([1, 2]);
    expect(filter.tag_ids).toEqual([1, 2]);
    expect(filter.sync_states).toEqual(["ok", "new"]);
  });

  it("builds rating, camera, metadata, gps, duplicate, and delete filters", () => {
    const filter = buildFilterFromBar({
      ...baseBar,
      ratingMin: 4,
      camera: "ILCE-7C",
      metaSearch: "  sunset ",
      hasGps: true,
      hasDuplicate: true,
      deleteStatus: "deleted",
      captureFrom: "2024-01-01",
      captureTo: "2024-12-31",
    });
    expect(filter.rating_min).toBe(4);
    expect(filter.camera).toBe("ILCE-7C");
    expect(filter.meta_search).toBe("sunset");
    expect(filter.has_gps).toBe(true);
    expect(filter.has_duplicate).toBe(true);
    expect(filter.deleted_only).toBe(true);
    expect(filter.capture_from).toBeDefined();
    expect(filter.capture_to).toBeDefined();
  });

  it("toolbar filters override stale extra filter scope", () => {
    const filter = mergeLibraryFilter(
      { ...baseBar, albumIds: [3] },
      { root_id: 2 },
    );
    expect(filter.album_ids).toEqual([3]);
    expect(filter.root_id).toBe(2);
  });

  it("keeps only scope fields from extra filter", () => {
    expect(
      assetFilterScope({
        root_id: 1,
        kind: "image",
        album_ids: [4],
        rating_min: 3,
      }),
    ).toEqual({ root_id: 1, kind: "image" });
  });

  it("roundtrips collection filters through filter bar state", () => {
    const bar = filterBarFromAssetFilter(
      {
        album_ids: [9],
        rating_min: 4,
        has_gps: true,
        capture_from: 1_700_000_000,
        capture_to: 1_800_000_000,
        deleted_only: true,
      },
      "date",
      "desc",
    );
    expect(bar.albumIds).toEqual([9]);
    expect(bar.ratingMin).toBe(4);
    expect(bar.hasGps).toBe(true);
    expect(bar.deleteStatus).toBe("deleted");
    expect(bar.captureFrom).not.toBe("");
    expect(bar.captureTo).not.toBe("");
    expect(buildFilterFromBar(bar).album_ids).toEqual([9]);
  });

  it("maps plural filter fields to filter bar state", () => {
    const bar = filterBarFromAssetFilter(
      {
        tag_ids: [2],
        sync_states: ["ok"],
        album_ids: [2, 3],
      },
      "name",
      "asc",
    );
    expect(bar.tagIds).toEqual([2]);
    expect(bar.syncStates).toEqual(["ok"]);
    expect(bar.albumIds).toEqual([2, 3]);
  });

  it("builds filter definitions from tags and albums", () => {
    const tags: TagDto[] = [
      {
        id: 1,
        name: "travel",
        parent_id: null,
        color: "#ff0000",
        asset_count: 4,
      },
    ];
    const albums: Album[] = [
      {
        id: 7,
        name: "Summer",
        sort_mode: "date:desc",
        emoji: "☀️",
        asset_count: 12,
      },
    ];
    const defs = buildLibraryFilterDefs(tags, albums);
    expect(defs.map((def) => def.id)).toEqual(FILTER_DISPLAY_ORDER);
    expect(
      defs.find((def) => def.id === FILTER_IDS.tag)?.statusOptions,
    ).toEqual(["1"]);
    expect(
      defs.find((def) => def.id === FILTER_IDS.album)?.statusOptionLabels?.[
        "7"
      ],
    ).toBe("☀️ Summer");
  });

  it("extracts active filter values from toolbar state", () => {
    const bar = {
      ...baseBar,
      ratingMin: 3,
      deleteStatus: "deleted" as const,
      camera: "Canon",
      metaSearch: "beach",
      hasGps: true,
      hasDuplicate: true,
      syncStates: ["ok"],
      tagIds: [1],
      albumIds: [5],
    };
    expect(filterValuesFromBar(bar)).toEqual({
      [FILTER_IDS.rating]: "3",
      [FILTER_IDS.delete]: "deleted",
      [FILTER_IDS.camera]: "Canon",
      [FILTER_IDS.metadata]: "beach",
      [FILTER_IDS.gps]: "1",
      [FILTER_IDS.duplicate]: "1",
    });
    expect(multiFilterValuesFromBar(bar)).toEqual({
      [FILTER_IDS.sync]: ["ok"],
      [FILTER_IDS.tag]: ["1"],
      [FILTER_IDS.album]: ["5"],
    });
  });

  it("applies filter changes through toolbar handlers", () => {
    expect(applyChange(baseBar, FILTER_IDS.rating, "4").ratingMin).toBe(4);
    expect(applyChange(baseBar, FILTER_IDS.rating, "").ratingMin).toBe("");
    expect(applyChange(baseBar, FILTER_IDS.sync, "ok").syncStates).toEqual([
      "ok",
    ]);
    expect(
      applyChange({ ...baseBar, syncStates: ["ok"] }, FILTER_IDS.sync, "ok")
        .syncStates,
    ).toEqual([]);
    expect(
      applyChange(baseBar, FILTER_IDS.delete, "deleted").deleteStatus,
    ).toBe("deleted");
    expect(applyChange(baseBar, FILTER_IDS.camera, "Nikon").camera).toBe(
      "Nikon",
    );
    expect(applyChange(baseBar, FILTER_IDS.tag, "2").tagIds).toEqual([2]);
    expect(
      applyChange({ ...baseBar, tagIds: [2] }, FILTER_IDS.tag, "2").tagIds,
    ).toEqual([]);
    expect(applyChange(baseBar, FILTER_IDS.album, "2").albumIds).toEqual([2]);
    expect(
      applyChange({ ...baseBar, albumIds: [2] }, FILTER_IDS.album, "2")
        .albumIds,
    ).toEqual([]);
    expect(applyChange(baseBar, FILTER_IDS.metadata, "sunset").metaSearch).toBe(
      "sunset",
    );
    expect(applyChange(baseBar, FILTER_IDS.gps, "1").hasGps).toBe(true);
    expect(applyChange(baseBar, FILTER_IDS.gps, "0").hasGps).toBe(false);
    expect(applyChange(baseBar, FILTER_IDS.duplicate, "1").hasDuplicate).toBe(
      true,
    );
    expect(applyChange(baseBar, "unknown", "x")).toEqual(baseBar);
  });

  it("removes filters through toolbar handlers", () => {
    const active = {
      ...baseBar,
      ratingMin: 5,
      syncStates: ["ok"],
      deleteStatus: "deleted" as const,
      camera: "Canon",
      tagIds: [2],
      albumIds: [1],
      metaSearch: "beach",
      hasGps: true,
      hasDuplicate: true,
      captureFrom: "2024-01-01",
      captureTo: "2024-12-31",
    };
    expect(removeFilter(active, FILTER_IDS.rating).ratingMin).toBe("");
    expect(removeFilter(active, FILTER_IDS.sync).syncStates).toEqual([]);
    expect(removeFilter(active, FILTER_IDS.delete).deleteStatus).toBe("");
    expect(removeFilter(active, FILTER_IDS.camera).camera).toBe("");
    expect(removeFilter(active, FILTER_IDS.tag).tagIds).toEqual([]);
    expect(removeFilter(active, FILTER_IDS.album).albumIds).toEqual([]);
    expect(removeFilter(active, FILTER_IDS.metadata).metaSearch).toBe("");
    expect(removeFilter(active, FILTER_IDS.capture).captureFrom).toBe("");
    expect(removeFilter(active, FILTER_IDS.capture).captureTo).toBe("");
    expect(removeFilter(active, FILTER_IDS.gps).hasGps).toBe(false);
    expect(removeFilter(active, FILTER_IDS.duplicate).hasDuplicate).toBe(false);
    expect(removeFilter(active, "unknown")).toEqual(active);
  });

  it("creates and clears empty filter bar state", () => {
    expect(emptyFilterBarState("name", "asc")).toEqual({
      ...baseBar,
      sort: "name",
      sortDir: "asc",
    });
    let cleared = baseBar;
    clearLibraryFilters(
      (update) => {
        cleared = typeof update === "function" ? update(baseBar) : update;
      },
      "rating",
      "asc",
    );
    expect(cleared).toEqual(emptyFilterBarState("rating", "asc"));
  });

  it("detects active root, album, and tag sources", () => {
    expect(isRootSourceActive(3, baseBar, { root_id: 3 })).toBe(true);
    expect(
      isRootSourceActive(3, { ...baseBar, ratingMin: 2 }, { root_id: 3 }),
    ).toBe(false);
    expect(
      isRootSourceActive(3, { ...baseBar, albumIds: [1] }, { root_id: 3 }),
    ).toBe(false);
    expect(isAlbumSourceActive(5, { ...baseBar, albumIds: [5] }, {})).toBe(
      true,
    );
    expect(isAlbumSourceActive(5, { ...baseBar, albumIds: [5, 6] }, {})).toBe(
      false,
    );
    expect(isTagSourceActive(1, { ...baseBar, tagIds: [1] }, {})).toBe(true);
    expect(isTagSourceActive(1, { ...baseBar, tagIds: [1, 2] }, {})).toBe(
      false,
    );
  });

  it("treats capture-only toolbar filters as active scope blockers", () => {
    expect(
      isRootSourceActive(
        3,
        { ...baseBar, captureTo: "2024-12-31" },
        { root_id: 3 },
      ),
    ).toBe(false);
  });

  it("rejects album and tag sources when conflicting filters are active", () => {
    expect(
      isAlbumSourceActive(
        5,
        { ...baseBar, albumIds: [5], deleteStatus: "deleted" },
        {},
      ),
    ).toBe(false);
    expect(
      isAlbumSourceActive(5, { ...baseBar, albumIds: [5], tagIds: [1] }, {}),
    ).toBe(false);
    expect(
      isAlbumSourceActive(
        5,
        { ...baseBar, albumIds: [5], hasDuplicate: true },
        {},
      ),
    ).toBe(false);
    expect(
      isTagSourceActive(
        1,
        { ...baseBar, tagIds: [1], deleteStatus: "deleted" },
        {},
      ),
    ).toBe(false);
    expect(
      isTagSourceActive(1, { ...baseBar, tagIds: [1], albumIds: [2] }, {}),
    ).toBe(false);
    expect(
      isTagSourceActive(1, { ...baseBar, tagIds: [1], hasDuplicate: true }, {}),
    ).toBe(false);
    expect(
      isAlbumSourceActive(5, { ...baseBar, albumIds: [5] }, { root_id: 1 }),
    ).toBe(false);
    expect(
      isTagSourceActive(1, { ...baseBar, tagIds: [1] }, { kind: "video" }),
    ).toBe(false);
  });
});
