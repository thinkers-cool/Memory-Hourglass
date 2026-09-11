import type React from "react";
import type { FilterBarState } from "../../lib/libraryActions";
import type { StampConfig } from "../../lib/stamp";
import type {
  AssetCard,
  AssetDetail,
  AssetFilter,
  Notification,
} from "../../types";

export type LibraryActionsDeps = {
  items: AssetCard[];
  selectedId: number | null;
  selectedIds: Set<number>;
  selectedList: number[];
  detail: AssetDetail | null;
  gridColumnCount: number;
  fullView: boolean;
  compareOpen: boolean;
  compareItems: AssetCard[];
  compareDetails: Record<number, { tag_ids: number[]; album_ids: number[] }>;
  stampConfig: StampConfig;
  stampArmed: boolean;
  disarmStamp: () => void;
  setStampRating: (rating: number | null) => void;
  toggleStampTagInConfig: (tagId: number, add: boolean) => void;
  toggleStampAlbumInConfig: (albumId: number, add: boolean) => void;
  markStampResults: (stampedIds: number[], unstampedIds: number[]) => void;
  purgeTargetIds: number[];
  readOnly: boolean;
  filterRef: React.MutableRefObject<AssetFilter>;
  sortParam: string;
  lastSelectedIndexRef: React.MutableRefObject<number | null>;
  setSelectedId: React.Dispatch<React.SetStateAction<number | null>>;
  setSelectedIds: React.Dispatch<React.SetStateAction<Set<number>>>;
  setDetail: React.Dispatch<React.SetStateAction<AssetDetail | null>>;
  setExtraFilter: React.Dispatch<React.SetStateAction<AssetFilter>>;
  setFilterBar: React.Dispatch<React.SetStateAction<FilterBarState>>;
  setSelectedCollectionId: React.Dispatch<React.SetStateAction<number | null>>;
  setFullView: React.Dispatch<React.SetStateAction<boolean>>;
  setInspectorVisible: React.Dispatch<React.SetStateAction<boolean>>;
  setGalleryIndex: React.Dispatch<React.SetStateAction<number | null>>;
  setCompareOpen: React.Dispatch<React.SetStateAction<boolean>>;
  setCompareIds: React.Dispatch<React.SetStateAction<number[]>>;
  setCompareItems: React.Dispatch<React.SetStateAction<AssetCard[]>>;
  setCompareDetails: React.Dispatch<
    React.SetStateAction<
      Record<number, { tag_ids: number[]; album_ids: number[] }>
    >
  >;
  setSmbDialogOpen: React.Dispatch<React.SetStateAction<boolean>>;
  setCollectionDialogOpen: React.Dispatch<React.SetStateAction<boolean>>;
  setTagMenuOpen: React.Dispatch<React.SetStateAction<boolean>>;
  setAlbumMenuOpen: React.Dispatch<React.SetStateAction<boolean>>;
  setPurgeDialogOpen: React.Dispatch<React.SetStateAction<boolean>>;
  setPurgeTargetIds: React.Dispatch<React.SetStateAction<number[]>>;
  setGridColumnCount: React.Dispatch<React.SetStateAction<number>>;
  setScanStatus: React.Dispatch<React.SetStateAction<string>>;
  setNotification: React.Dispatch<React.SetStateAction<Notification | null>>;
  refreshMeta: () => Promise<void>;
  refreshGrid: () => Promise<void>;
  refreshAll: () => Promise<void>;
  loadMore: () => Promise<void>;
  withBusy: (fn: () => Promise<void>) => Promise<void>;
  openExport: (assetIds: number[]) => void;
  requestConfirm: (
    title: string,
    message: string,
    onConfirm: () => void | Promise<void>,
  ) => void;
  addRootAndScan: (
    add: (path: string) => Promise<{ id: number; path: string }>,
  ) => Promise<void>;
};
