import type { ExportProgressState } from "./jobProgress";
import type { SmartCollection } from "../types";
import type { AssetCard } from "../types";
import type { ExportOptions } from "../types";
import type { SortMode, SortDir } from "../types";

export type SmbConnectParams = {
  host: string;
  share: string;
  username: string;
  password: string;
  pollSecs: number;
  folderPath?: string;
};

export type LibraryActions = {
  addLocalRoot: () => void;
  openSmbConnect: () => void;
  connectSmbShare: (params: SmbConnectParams) => void;
  addMountedSmbPath: (path: string) => void;
  removeRoot: (id: number) => void;
  syncRoot: (id: number) => void;
  selectRoot: (id: number) => void;
  clearFilters: () => void;
  openExport: (assetIds: number[]) => void;
  createAlbum: (name: string, emoji?: string) => void;
  deleteAlbum: (id: number) => void;
  selectAlbum: (id: number) => void;
  selectCollection: (sc: SmartCollection) => void;
  deleteCollection: (id: number) => void;
  relinkRoot: (id: number) => void;
  selectAsset: (card: AssetCard, multi: boolean, range: boolean) => void;
  openLinked: (id: number) => void;
  rate: (rating: number) => void;
  batchRate: (rating: number) => void;
  openTagMenu: () => void;
  openAlbumMenu: () => void;
  toggleTagOnSelection: (tagId: number, add: boolean) => void;
  createTagOnSelection: (
    tagName: string,
  ) => string | void | Promise<string | void>;
  toggleAlbumOnSelection: (albumId: number, add: boolean) => void;
  createAlbumOnSelection: (
    name: string,
  ) => string | void | Promise<string | void>;
  batchRemove: () => void;
  batchPurge: () => void;
  softDeleteDuplicate: (id: number) => void;
  softDelete: () => void;
  purge: () => void;
  submitPurge: () => void;
  restoreSelected: () => void;
  saveCollection: () => void;
  submitSaveCollection: (name: string) => void;
  openGallery: () => void;
  closeDetail: () => void;
  closeInspector: () => void;
  clearSelection: () => void;
  loadMore: () => void;
  filterByTag: (tagId: number) => void;
  createTag: (name: string, parentId?: number, color?: string) => void;
  updateTag: (id: number, name: string, color?: string) => void;
  deleteTag: (id: number) => void;
  updateAlbum: (id: number, name: string, emoji?: string) => void;
  viewTrash: () => void;
  viewLibrary: () => void;
  rebuildCatalog: () => void;
  cancelScan: () => void;
  navigateRelative: (delta: number) => void;
  navigateGrid: (deltaCol: number, deltaRow: number) => void;
  adjustGridSize: (delta: number) => void;
  openFullView: () => void;
  openFullViewFor: (card: AssetCard) => void;
  closeFullView: () => void;
  toggleFullscreen: () => void;
  openCompare: () => void;
  closeCompare: () => void;
  rateAsset: (id: number, rating: number) => void;
  toggleTagOnAsset: (
    id: number,
    tagId: number,
    add: boolean,
  ) => void | Promise<void>;
  createTagOnAsset: (
    id: number,
    tagName: string,
  ) => string | void | Promise<string | void>;
  toggleAlbumOnAsset: (
    id: number,
    albumId: number,
    add: boolean,
  ) => void | Promise<void>;
  createAlbumOnAsset: (
    id: number,
    name: string,
  ) => string | void | Promise<string | void>;
  deleteAsset: (id: number) => void;
  disarmStamp: () => void;
  setStampRating: (rating: number | null) => void;
  toggleStampTag: (tagId: number, add: boolean) => void;
  toggleStampAlbum: (albumId: number, add: boolean) => void;
  toggleStampOnTargets: () => void;
};

export type ExportDialogState = {
  open: boolean;
  assetIds: number[];
  destination: string;
  options: ExportOptions;
  jobId: number | null;
  progress: ExportProgressState;
};

export const DEFAULT_EXPORT_OPTIONS: ExportOptions = {
  flat: true,
  rename_template: undefined,
  format: undefined,
};

export type FilterBarState = {
  ratingMin: number | "";
  syncStates: string[];
  deleteStatus: "" | "deleted";
  camera: string;
  tagIds: number[];
  albumIds: number[];
  metaSearch: string;
  hasGps: boolean;
  hasDuplicate: boolean;
  captureFrom: string;
  captureTo: string;
  sort: SortMode;
  sortDir: SortDir;
};
