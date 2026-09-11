import { vi } from "vitest";
import { EMPTY_STAMP_CONFIG } from "../lib/stamp";
import type { AssetCard, AssetDetail } from "../types";
import type { LibraryActionsDeps } from "../hooks/library/createLibraryActions";

export const sampleActionItem: AssetCard = {
  id: 1,
  file_name: "photo.jpg",
  ext: "jpg",
  kind: "image",
  capture_at: null,
  rating: null,
  sync_state: "ok",
  thumb_path: null,
  abs_path: "/tmp/1.jpg",
  has_duplicate: false,
};

export const sampleActionItem2: AssetCard = {
  ...sampleActionItem,
  id: 2,
  file_name: "photo2.jpg",
  abs_path: "/tmp/2.jpg",
};

export const sampleActionDetail: AssetDetail = {
  asset: {
    id: 1,
    root_id: 1,
    rel_path: "photo.jpg",
    file_name: "photo.jpg",
    ext: "jpg",
    kind: "image",
    size: 1024,
    mtime_ns: 0,
    sync_state: "ok",
  },
  meta: null,
  abs_path: "/tmp/1.jpg",
  display_path: "/tmp/1.jpg",
  tag_ids: [7],
  album_ids: [8],
  raw_tags: [],
  links: [],
  duplicates: [],
};

export function makeLibraryActionsDeps(
  overrides: Partial<LibraryActionsDeps> = {},
): LibraryActionsDeps & {
  setNotification: ReturnType<typeof vi.fn>;
  requestConfirm: ReturnType<typeof vi.fn>;
  disarmStamp: ReturnType<typeof vi.fn>;
  markStampResults: ReturnType<typeof vi.fn>;
  withBusy: (fn: () => Promise<void>) => Promise<void>;
} {
  const setNotification = vi.fn();
  const disarmStamp = vi.fn();
  const markStampResults = vi.fn();
  const requestConfirm = vi.fn();

  return {
    items: [sampleActionItem, sampleActionItem2],
    selectedId: 1,
    selectedIds: new Set([1]),
    selectedList: [1],
    detail: sampleActionDetail,
    gridColumnCount: 5,
    fullView: false,
    compareOpen: false,
    compareItems: [],
    compareDetails: {},
    stampConfig: { ...EMPTY_STAMP_CONFIG, rating: 3, tag_ids: [7], album_ids: [8] },
    stampArmed: true,
    disarmStamp,
    setStampRating: vi.fn(),
    toggleStampTagInConfig: vi.fn(),
    toggleStampAlbumInConfig: vi.fn(),
    markStampResults,
    purgeTargetIds: [],
    readOnly: false,
    filterRef: { current: {} },
    sortParam: "date:desc",
    lastSelectedIndexRef: { current: 0 },
    setSelectedId: vi.fn(),
    setSelectedIds: vi.fn(),
    setDetail: vi.fn(),
    setExtraFilter: vi.fn(),
    setFilterBar: vi.fn(),
    setSelectedCollectionId: vi.fn(),
    setFullView: vi.fn(),
    setInspectorVisible: vi.fn(),
    setGalleryIndex: vi.fn(),
    setCompareOpen: vi.fn(),
    setCompareIds: vi.fn(),
    setCompareItems: vi.fn(),
    setCompareDetails: vi.fn(),
    setSmbDialogOpen: vi.fn(),
    setCollectionDialogOpen: vi.fn(),
    setTagMenuOpen: vi.fn(),
    setAlbumMenuOpen: vi.fn(),
    setPurgeDialogOpen: vi.fn(),
    setPurgeTargetIds: vi.fn(),
    setGridColumnCount: vi.fn(),
    setScanStatus: vi.fn(),
    setNotification,
    refreshMeta: vi.fn().mockResolvedValue(undefined),
    refreshGrid: vi.fn().mockResolvedValue(undefined),
    refreshAll: vi.fn().mockResolvedValue(undefined),
    loadMore: vi.fn().mockResolvedValue(undefined),
    withBusy: async (fn: () => Promise<void>) => {
      await fn();
    },
    openExport: vi.fn(),
    requestConfirm,
    addRootAndScan: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  } as LibraryActionsDeps & {
    setNotification: ReturnType<typeof vi.fn>;
    requestConfirm: ReturnType<typeof vi.fn>;
    disarmStamp: ReturnType<typeof vi.fn>;
    markStampResults: ReturnType<typeof vi.fn>;
    withBusy: (fn: () => Promise<void>) => Promise<void>;
  };
}
