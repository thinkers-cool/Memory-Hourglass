import type { Album, RootStats, TagDto } from "../types";
import { compareTagNames } from "./tagHierarchy";
import { loadStoredJson, saveStoredJson } from "./storage";

function rootLabel(path: string): string {
  const normalized = path.replace(/[/\\]+$/, "");
  const segments = normalized.split(/[/\\]/).filter(Boolean);
  return segments[segments.length - 1] ?? path;
}

export type PanelSectionKey = "roots" | "albums" | "tags";
export type PanelOrderMode = "name-asc" | "name-desc";

export interface PanelOrderStore {
  roots: PanelOrderMode;
  albums: PanelOrderMode;
  tags: PanelOrderMode;
}

export const DEFAULT_PANEL_ORDER_STORE: PanelOrderStore = {
  roots: "name-asc",
  albums: "name-asc",
  tags: "name-asc",
};

export function panelOrderStorageKey(workspaceId: string): string {
  return `memhg.panelOrder:v1:${workspaceId}`;
}

function isPanelOrderMode(value: unknown): value is PanelOrderMode {
  return value === "name-asc" || value === "name-desc";
}

function parseSectionMode(value: unknown): PanelOrderMode {
  if (typeof value === "string" && isPanelOrderMode(value)) {
    return value;
  }
  if (value && typeof value === "object") {
    const mode = (value as Record<string, unknown>).mode;
    if (isPanelOrderMode(mode)) {
      return mode;
    }
  }
  return "name-asc";
}

export function parsePanelOrderStore(value: unknown): PanelOrderStore | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const record = value as Record<string, unknown>;
  return {
    roots: parseSectionMode(record.roots),
    albums: parseSectionMode(record.albums),
    tags: parseSectionMode(record.tags),
  };
}

export function loadPanelOrderStore(workspaceId: string): PanelOrderStore {
  return loadStoredJson(
    panelOrderStorageKey(workspaceId),
    DEFAULT_PANEL_ORDER_STORE,
    parsePanelOrderStore,
  );
}

export function savePanelOrderStore(
  workspaceId: string,
  store: PanelOrderStore,
): void {
  saveStoredJson(panelOrderStorageKey(workspaceId), store);
}

export function togglePanelOrderMode(mode: PanelOrderMode): PanelOrderMode {
  return mode === "name-asc" ? "name-desc" : "name-asc";
}

function sortByName<T>(
  items: T[],
  mode: PanelOrderMode,
  compare: (a: T, b: T) => number,
): T[] {
  if (mode === "name-desc") {
    return [...items].sort((a, b) => compare(b, a));
  }
  return [...items].sort(compare);
}

function compareRootNames(a: RootStats, b: RootStats): number {
  return rootLabel(a.path).localeCompare(rootLabel(b.path));
}

function compareAlbumNames(a: Album, b: Album): number {
  return a.name.localeCompare(b.name);
}

export function orderRoots(
  roots: RootStats[],
  mode: PanelOrderMode,
): RootStats[] {
  return sortByName(roots, mode, compareRootNames);
}

export function orderAlbums(albums: Album[], mode: PanelOrderMode): Album[] {
  return sortByName(albums, mode, compareAlbumNames);
}

export function orderTagsForParent(
  tags: TagDto[],
  parentId: number | null,
  mode: PanelOrderMode,
): TagDto[] {
  const children = tags.filter((tag) => tag.parent_id === parentId);
  return sortByName(children, mode, compareTagNames);
}
