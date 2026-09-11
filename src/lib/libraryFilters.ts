import type { Dispatch, SetStateAction } from "react";
import type { FilterDef } from "../components/shared/FilterChipBar";
import type { FilterBarState } from "./libraryActions";
import type { Album, AssetFilter, TagDto } from "../types";
import i18n from "../i18n";
import { albumEmoji } from "./libraryIndicators";

export function dateInputToUnix(value: string): number | undefined {
  if (!value) return undefined;
  const ms = Date.parse(`${value}T00:00:00`);
  if (Number.isNaN(ms)) return undefined;
  return Math.floor(ms / 1000);
}

export function dateInputEndToUnix(value: string): number | undefined {
  if (!value) return undefined;
  const ms = Date.parse(`${value}T23:59:59`);
  if (Number.isNaN(ms)) return undefined;
  return Math.floor(ms / 1000);
}

export function isValidCaptureUnix(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value > 0;
}

export function unixToDateInput(value?: number | null): string {
  if (!isValidCaptureUnix(value)) return "";
  const date = new Date(value * 1000);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function filterBarFromAssetFilter(
  filter: AssetFilter,
  sort: FilterBarState["sort"],
  sortDir: FilterBarState["sortDir"],
): FilterBarState {
  return {
    ratingMin: filter.rating_min ?? "",
    syncStates: filter.sync_states ?? [],
    deleteStatus: filter.deleted_only ? "deleted" : "",
    camera: filter.camera ?? "",
    tagIds: filter.tag_ids ?? [],
    albumIds: filter.album_ids ?? [],
    metaSearch: filter.meta_search ?? "",
    hasGps: filter.has_gps ?? false,
    hasDuplicate: filter.has_duplicate ?? false,
    captureFrom: isValidCaptureUnix(filter.capture_from)
      ? unixToDateInput(filter.capture_from)
      : "",
    captureTo: isValidCaptureUnix(filter.capture_to)
      ? unixToDateInput(filter.capture_to)
      : "",
    sort,
    sortDir,
  };
}
export function assetFilterScope(filter: AssetFilter): AssetFilter {
  const scope: AssetFilter = {};
  if (filter.root_id !== undefined) scope.root_id = filter.root_id;
  if (filter.kind !== undefined) scope.kind = filter.kind;
  return scope;
}

export function buildFilterFromBar(filterBar: FilterBarState): AssetFilter {
  const filter: AssetFilter = {};
  if (filterBar.ratingMin !== "") filter.rating_min = Number(filterBar.ratingMin);
  if (filterBar.syncStates.length > 0) filter.sync_states = [...filterBar.syncStates];
  if (filterBar.camera) filter.camera = filterBar.camera;
  if (filterBar.tagIds.length > 0) filter.tag_ids = [...filterBar.tagIds];
  if (filterBar.albumIds.length > 0) {
    filter.album_ids = [...filterBar.albumIds];
  }
  if (filterBar.metaSearch.trim()) filter.meta_search = filterBar.metaSearch.trim();
  if (filterBar.hasGps) filter.has_gps = true;
  if (filterBar.hasDuplicate) filter.has_duplicate = true;
  const from = dateInputToUnix(filterBar.captureFrom);
  const to = dateInputEndToUnix(filterBar.captureTo);
  if (isValidCaptureUnix(from)) filter.capture_from = from;
  if (isValidCaptureUnix(to)) filter.capture_to = to;
  if (filterBar.deleteStatus === "deleted") filter.deleted_only = true;
  return filter;
}

export function mergeLibraryFilter(
  filterBar: FilterBarState,
  extraFilter: AssetFilter,
): AssetFilter {
  return { ...assetFilterScope(extraFilter), ...buildFilterFromBar(filterBar) };
}

function toggleListValue(values: string[], value: string): string[] {
  const next = new Set(values);
  if (next.has(value)) next.delete(value);
  else next.add(value);
  return Array.from(next);
}

function toggleTagId(values: number[], value: number): number[] {
  const next = new Set(values);
  if (next.has(value)) next.delete(value);
  else next.add(value);
  return Array.from(next);
}

function toggleAlbumId(values: number[], value: number): number[] {
  const next = new Set(values);
  if (next.has(value)) next.delete(value);
  else next.add(value);
  return Array.from(next);
}

export const FILTER_IDS = {
  rating: "rating",
  sync: "sync",
  delete: "delete",
  camera: "camera",
  tag: "tag",
  album: "album",
  metadata: "metadata",
  capture: "capture",
  gps: "gps",
  duplicate: "duplicate",
} as const;

export type DeleteFilterStatus = "" | "deleted";

export function buildLibraryFilterDefs(
  tags: TagDto[],
  albums: Album[],
): FilterDef[] {
  const albumLabels = Object.fromEntries(
    albums.map((album) => [String(album.id), `${albumEmoji(album.emoji)} ${album.name}`]),
  );
  const tagLabels = Object.fromEntries(
    tags.map((tag) => [String(tag.id), tag.name]),
  );

  return [
    {
      id: FILTER_IDS.rating,
      label: i18n.t("library:filters.rating"),
      type: "status",
      statusOptions: ["1", "3", "5"],
      statusOptionLabels: {
        "1": i18n.t("library:filters.ratingMin.1"),
        "3": i18n.t("library:filters.ratingMin.3"),
        "5": i18n.t("library:filters.ratingMin.5"),
      },
    },
    {
      id: FILTER_IDS.sync,
      label: i18n.t("library:filters.sync"),
      type: "status",
      multi: true,
      statusOptions: ["ok", "new", "modified", "missing"],
      statusOptionLabels: {
        ok: i18n.t("library:filters.syncStatus.ok"),
        new: i18n.t("library:filters.syncStatus.new"),
        modified: i18n.t("library:filters.syncStatus.modified"),
        missing: i18n.t("library:filters.syncStatus.missing"),
      },
    },
    {
      id: FILTER_IDS.delete,
      label: i18n.t("library:filters.delete"),
      type: "status",
      statusOptions: ["deleted"],
      statusOptionLabels: {
        deleted: i18n.t("library:filters.deleteStatus.deleted"),
      },
    },
    {
      id: FILTER_IDS.camera,
      label: i18n.t("library:filters.camera"),
      type: "text",
    },
    {
      id: FILTER_IDS.tag,
      label: i18n.t("library:filters.tag"),
      type: "status",
      multi: true,
      statusOptions: tags.map((tag) => String(tag.id)),
      statusOptionLabels: tagLabels,
    },
    {
      id: FILTER_IDS.album,
      label: i18n.t("library:filters.album"),
      type: "status",
      multi: true,
      statusOptions: albums.map((album) => String(album.id)),
      statusOptionLabels: albumLabels,
    },
    {
      id: FILTER_IDS.metadata,
      label: i18n.t("library:filters.metadata"),
      type: "text",
      debounceMs: 300,
    },
    {
      id: FILTER_IDS.capture,
      label: i18n.t("library:filters.date"),
      type: "date",
    },
    {
      id: FILTER_IDS.gps,
      label: i18n.t("library:filters.gps"),
      type: "status",
      statusOptions: ["1"],
      statusOptionLabels: { "1": i18n.t("library:filters.gpsHasLocation") },
    },
    {
      id: FILTER_IDS.duplicate,
      label: i18n.t("library:filters.duplicate"),
      type: "status",
      statusOptions: ["1"],
      statusOptionLabels: { "1": i18n.t("library:filters.duplicateHas") },
    },
  ];
}

export function filterValuesFromBar(bar: FilterBarState): Record<string, string> {
  const values: Record<string, string> = {};
  if (bar.ratingMin !== "") values[FILTER_IDS.rating] = String(bar.ratingMin);
  if (bar.deleteStatus) values[FILTER_IDS.delete] = bar.deleteStatus;
  if (bar.camera) values[FILTER_IDS.camera] = bar.camera;
  if (bar.metaSearch) values[FILTER_IDS.metadata] = bar.metaSearch;
  if (bar.hasGps) values[FILTER_IDS.gps] = "1";
  if (bar.hasDuplicate) values[FILTER_IDS.duplicate] = "1";
  return values;
}

export function multiFilterValuesFromBar(
  bar: FilterBarState,
): Record<string, string[]> {
  const values: Record<string, string[]> = {};
  if (bar.syncStates.length > 0) values[FILTER_IDS.sync] = bar.syncStates;
  if (bar.tagIds.length > 0) {
    values[FILTER_IDS.tag] = bar.tagIds.map(String);
  }
  if (bar.albumIds.length > 0) {
    values[FILTER_IDS.album] = bar.albumIds.map(String);
  }
  return values;
}

export function applyLibraryFilterChange(
  id: string,
  value: string,
  setFilterBar: Dispatch<SetStateAction<FilterBarState>>,
) {
  switch (id) {
    case FILTER_IDS.rating:
      setFilterBar((prev) => ({
        ...prev,
        ratingMin: value ? Number(value) : "",
      }));
      break;
    case FILTER_IDS.sync:
      setFilterBar((prev) => ({ ...prev, syncStates: toggleListValue(prev.syncStates, value) }));
      break;
    case FILTER_IDS.delete:
      setFilterBar((prev) => ({
        ...prev,
        deleteStatus: value as DeleteFilterStatus,
      }));
      break;
    case FILTER_IDS.camera:
      setFilterBar((prev) => ({ ...prev, camera: value }));
      break;
    case FILTER_IDS.tag:
      setFilterBar((prev) => ({
        ...prev,
        tagIds: toggleTagId(prev.tagIds, Number(value)),
      }));
      break;
    case FILTER_IDS.album:
      setFilterBar((prev) => ({
        ...prev,
        albumIds: toggleAlbumId(prev.albumIds, Number(value)),
      }));
      break;
    case FILTER_IDS.metadata:
      setFilterBar((prev) => ({ ...prev, metaSearch: value }));
      break;
    case FILTER_IDS.gps:
      setFilterBar((prev) => ({ ...prev, hasGps: value === "1" }));
      break;
    case FILTER_IDS.duplicate:
      setFilterBar((prev) => ({ ...prev, hasDuplicate: value === "1" }));
      break;
    default:
      break;
  }
}

export function removeLibraryFilter(
  id: string,
  setFilterBar: Dispatch<SetStateAction<FilterBarState>>,
) {
  switch (id) {
    case FILTER_IDS.rating:
      setFilterBar((prev) => ({ ...prev, ratingMin: "" }));
      break;
    case FILTER_IDS.sync:
      setFilterBar((prev) => ({ ...prev, syncStates: [] }));
      break;
    case FILTER_IDS.delete:
      setFilterBar((prev) => ({ ...prev, deleteStatus: "" }));
      break;
    case FILTER_IDS.camera:
      setFilterBar((prev) => ({ ...prev, camera: "" }));
      break;
    case FILTER_IDS.tag:
      setFilterBar((prev) => ({ ...prev, tagIds: [] }));
      break;
    case FILTER_IDS.album:
      setFilterBar((prev) => ({ ...prev, albumIds: [] }));
      break;
    case FILTER_IDS.metadata:
      setFilterBar((prev) => ({ ...prev, metaSearch: "" }));
      break;
    case FILTER_IDS.capture:
      setFilterBar((prev) => ({
        ...prev,
        captureFrom: "",
        captureTo: "",
      }));
      break;
    case FILTER_IDS.gps:
      setFilterBar((prev) => ({ ...prev, hasGps: false }));
      break;
    case FILTER_IDS.duplicate:
      setFilterBar((prev) => ({ ...prev, hasDuplicate: false }));
      break;
    default:
      break;
  }
}

export function emptyFilterBarState(
  sort: FilterBarState["sort"],
  sortDir: FilterBarState["sortDir"],
): FilterBarState {
  return {
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
    sort,
    sortDir,
  };
}

export function clearLibraryFilters(
  setFilterBar: Dispatch<SetStateAction<FilterBarState>>,
  sort: FilterBarState["sort"],
  sortDir: FilterBarState["sortDir"],
) {
  setFilterBar(emptyFilterBarState(sort, sortDir));
}

function hasToolbarFilters(bar: FilterBarState): boolean {
  return (
    bar.ratingMin !== "" ||
    bar.syncStates.length > 0 ||
    bar.camera !== "" ||
    bar.metaSearch.trim() !== "" ||
    bar.hasGps ||
    bar.hasDuplicate ||
    bar.captureFrom !== "" ||
    bar.captureTo !== ""
  );
}

export function isRootSourceActive(
  rootId: number,
  filterBar: FilterBarState,
  extraFilter: AssetFilter,
): boolean {
  if (hasToolbarFilters(filterBar)) return false;
  if (
    filterBar.albumIds.length > 0 ||
    filterBar.tagIds.length > 0 ||
    filterBar.deleteStatus ||
    filterBar.hasDuplicate
  ) {
    return false;
  }
  const scope = assetFilterScope(extraFilter);
  if (scope.root_id !== rootId) return false;
  return Object.keys(scope).length === 1;
}

export function isAlbumSourceActive(
  albumId: number,
  filterBar: FilterBarState,
  extraFilter: AssetFilter,
): boolean {
  if (hasToolbarFilters(filterBar)) return false;
  if (filterBar.deleteStatus || filterBar.tagIds.length > 0 || filterBar.hasDuplicate) {
    return false;
  }
  if (Object.keys(assetFilterScope(extraFilter)).length > 0) return false;
  return (
    filterBar.albumIds.length === 1 && filterBar.albumIds[0] === albumId
  );
}

export function isTagSourceActive(
  tagId: number,
  filterBar: FilterBarState,
  extraFilter: AssetFilter,
): boolean {
  if (hasToolbarFilters(filterBar)) return false;
  if (
    filterBar.deleteStatus ||
    filterBar.albumIds.length > 0 ||
    filterBar.hasDuplicate
  ) {
    return false;
  }
  if (Object.keys(assetFilterScope(extraFilter)).length > 0) return false;
  return filterBar.tagIds.length === 1 && filterBar.tagIds[0] === tagId;
}
