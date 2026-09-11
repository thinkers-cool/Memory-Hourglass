import * as api from "../../api/client";
import i18n from "../../i18n";
import {
  emptyFilterBarState,
  assetFilterScope,
  filterBarFromAssetFilter,
} from "../../lib/libraryFilters";
import {
  runAlbumCreate,
  runAlbumToggle,
  runTagCreate,
  runTagToggle,
} from "../../lib/libraryMutations";
import { runStampToggle } from "../../lib/stampMutations";
import {
  assetSnapshotFromDetail,
  assetSnapshotFromParts,
  isStampConfigValid,
  resolveStampTargets,
  stampReferencesAlbum,
  stampReferencesTag,
  type StampConfig,
} from "../../lib/stamp";
import type { SmbConnectParams, FilterBarState } from "../../lib/libraryActions";
import {
  infoNotification,
  successNotification,
  errorNotification,
} from "../../lib/notification";
import { pickFolder } from "../../lib/pickFolder";
import { toggleSelection, applyRangeSelection, resolveSelectionAnchor } from "../../lib/selection";
import { navigateGridIndex } from "../../lib/gridNavigation";
import {
  clampGridColumnCount,
  GRID_COLUMN_COUNT_STEP,
} from "../../lib/gridSettings";
import type {
  AssetCard,
  AssetDetail,
  AssetFilter,
  Notification,
  SmartCollection,
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
    React.SetStateAction<Record<number, { tag_ids: number[]; album_ids: number[] }>>
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
  requestConfirm: (title: string, message: string, onConfirm: () => void | Promise<void>) => void;
  addRootAndScan: (
    add: (path: string) => Promise<{ id: number; path: string }>,
  ) => Promise<void>;
};

export function createLibraryActions(deps: LibraryActionsDeps) {
  const {
    items,
    selectedId,
    selectedIds,
    selectedList,
    detail,
    gridColumnCount,
    fullView,
    compareOpen,
    compareItems,
    compareDetails,
    stampConfig,
    stampArmed,
    disarmStamp,
    setStampRating,
    toggleStampTagInConfig,
    toggleStampAlbumInConfig,
    markStampResults,
    purgeTargetIds,
    readOnly,
    filterRef,
    sortParam,
    lastSelectedIndexRef,
    setSelectedId,
    setSelectedIds,
    setDetail,
    setExtraFilter,
    setFilterBar,
    setSelectedCollectionId,
    setFullView,
    setInspectorVisible,
    setGalleryIndex,
    setCompareOpen,
    setCompareIds,
    setCompareItems,
    setCompareDetails,
    setSmbDialogOpen,
    setCollectionDialogOpen,
    setTagMenuOpen,
    setAlbumMenuOpen,
    setPurgeDialogOpen,
    setPurgeTargetIds,
    setGridColumnCount,
    setScanStatus,
    setNotification,
    refreshMeta,
    refreshGrid,
    refreshAll,
    loadMore,
    withBusy,
    openExport,
    requestConfirm,
    addRootAndScan,
  } = deps;

  return {
    addLocalRoot: () => addRootAndScan(api.addRoot),
    openSmbConnect: () => setSmbDialogOpen(true),
    addMountedSmbPath: async (path: string) => {
      setNotification(infoNotification(i18n.t("library:notification.addingSmbFolder")));
      try {
        const root = await api.addSmbSource({
          mode: "mounted",
          path,
          poll_secs: 300,
        });
        await refreshMeta();
        setNotification(infoNotification(i18n.t("library:notification.addedSmbFolder")));
        void api.startScan(root.id).catch((error) => {
          setNotification(errorNotification(error));
        });
      } catch (error) {
        setNotification(errorNotification(error));
      }
    },
    connectSmbShare: async (params: SmbConnectParams) => {
      setSmbDialogOpen(false);
      setNotification(infoNotification(i18n.t("library:notification.connectingSmbShare")));
      try {
        const root = await api.connectSmbShare({
          host: params.host,
          share: params.share,
          username: params.username,
          password: params.password,
          pollSecs: params.pollSecs,
          subPath: params.folderPath,
        });
        await refreshMeta();
        setNotification(infoNotification(i18n.t("library:notification.smbShareConnected")));
        void api.startScan(root.id).catch((error) => {
          setNotification(errorNotification(error));
        });
      } catch (error) {
        setNotification(errorNotification(error));
      }
    },
    removeRoot: async (id: number) => {
      await api.removeRoot(id);
      await refreshAll();
    },
    syncRoot: async (rootId: number) => {
      setNotification(infoNotification(i18n.t("library:notification.scanning")));
      void api.startScan(rootId).catch((error) => {
        setNotification(errorNotification(error));
      });
    },
    selectRoot: (id: number) => {
      setSelectedCollectionId(null);
      setFilterBar((prev) => emptyFilterBarState(prev.sort, prev.sortDir));
      setExtraFilter({ root_id: id });
    },
    clearFilters: () => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => emptyFilterBarState(prev.sort, prev.sortDir));
    },
    openExport,
    selectAsset: async (card: AssetCard, multi: boolean, range: boolean) => {
      const index = items.findIndex((item) => item.id === card.id);
      if (index < 0) return;

      const anchorIndex = resolveSelectionAnchor(
        items,
        selectedId,
        lastSelectedIndexRef.current,
      );
      let nextIds: Set<number>;
      if (range && anchorIndex !== null) {
        nextIds = applyRangeSelection(
          items,
          anchorIndex,
          index,
          selectedIds,
          multi,
        );
      } else if (multi) {
        nextIds = toggleSelection(selectedIds, card.id, true);
      } else {
        nextIds = toggleSelection(selectedIds, card.id, false);
      }

      if (!range) {
        lastSelectedIndexRef.current = index;
      }
      setSelectedIds(nextIds);
      if (nextIds.size === 0) {
        setSelectedId(null);
        setDetail(null);
        lastSelectedIndexRef.current = null;
        return;
      }
      const focusId = nextIds.has(card.id)
        ? card.id
        : nextIds.values().next().value!;
      setSelectedId(focusId);
      setInspectorVisible(true);
      setDetail(await api.getAsset(focusId));
    },
    openFullViewFor: async (card: AssetCard) => {
      setSelectedIds(new Set([card.id]));
      setSelectedId(card.id);
      setInspectorVisible(true);
      setDetail(await api.getAsset(card.id));
      setFullView(true);
    },
    openFullView: async () => {
      if (selectedId === null) {
        if (items.length === 0) return;
        const card = items[0];
        setSelectedIds(new Set([card.id]));
        setSelectedId(card.id);
        setDetail(await api.getAsset(card.id));
      }
      setInspectorVisible(true);
      setFullView(true);
    },
    closeFullView: () => {
      setFullView(false);
      setInspectorVisible(true);
    },
    openLinked: async (id: number) => {
      setSelectedId(id);
      setInspectorVisible(true);
      setDetail(await api.getAsset(id));
    },
    rate: async (rating: number) => {
      if (!selectedId) return;
      setDetail(await api.updateAssetMeta(selectedId, { rating }));
      await refreshGrid();
    },
    batchRate: async (rating: number) => {
      if (selectedList.length === 0) return;
      await api.batchUpdateAssetMeta(selectedList, { rating });
      await refreshGrid();
    },
    openTagMenu: () => {
      if (selectedList.length === 0) return;
      setTagMenuOpen(true);
    },
    openAlbumMenu: () => {
      if (selectedList.length === 0) return;
      setAlbumMenuOpen(true);
    },
    toggleTagOnSelection: async (tagId: number, add: boolean) => {
      if (selectedList.length === 0) return;
      await withBusy(async () => {
        const count = await runTagToggle(selectedList, tagId, add);
        await refreshGrid();
        await refreshMeta();
        if (selectedId !== null && selectedIds.has(selectedId)) {
          setDetail(await api.getAsset(selectedId));
        }
      });
    },
    createTagOnSelection: async (tagName: string) => {
      const trimmed = tagName.trim();
      if (!trimmed || selectedList.length === 0) return;
      let tagId = 0;
      await withBusy(async () => {
        tagId = await runTagCreate(selectedList, trimmed);
        await refreshMeta();
        await refreshGrid();
        if (selectedId !== null && selectedIds.has(selectedId)) {
          setDetail(await api.getAsset(selectedId));
        }
      });
      return tagId > 0 ? String(tagId) : undefined;
    },
    batchRemove: async () => {
      if (selectedList.length === 0) return;
      await api.softDeleteAssets(selectedList);
      setSelectedIds(new Set());
      setSelectedId(null);
      setDetail(null);
      await refreshGrid();
    },
    softDelete: async () => {
      if (!selectedId) return;
      await api.softDeleteAssets([selectedId]);
      setSelectedId(null);
      setDetail(null);
      await refreshGrid();
    },
    purge: () => {
      if (readOnly || !selectedId) return;
      setPurgeTargetIds([selectedId]);
      setPurgeDialogOpen(true);
    },
    batchPurge: () => {
      if (readOnly || selectedList.length === 0) return;
      setPurgeTargetIds(selectedList);
      setPurgeDialogOpen(true);
    },
    submitPurge: async () => {
      if (readOnly || purgeTargetIds.length === 0) return;
      const purgedCount = purgeTargetIds.length;
      await withBusy(async () => {
        await api.purgeDelete(purgeTargetIds, "DELETE");
        setPurgeDialogOpen(false);
        setPurgeTargetIds([]);
        setSelectedIds(new Set());
        setSelectedId(null);
        setDetail(null);
        await refreshGrid();
        setNotification(
          successNotification(
            purgedCount === 1
              ? i18n.t("library:notification.filePurged")
              : i18n.t("library:notification.purged", { count: purgedCount }),
          ),
        );
      });
    },
    restoreSelected: async () => {
      const ids =
        selectedList.length > 0
          ? selectedList
          : selectedId
            ? [selectedId]
            : [];
      if (ids.length === 0) return;
      const count = await api.restoreAssets(ids);
      setSelectedIds(new Set());
      setSelectedId(null);
      setDetail(null);
      await refreshGrid();
      setNotification(successNotification(i18n.t("library:notification.restored", { count })));
    },
    createAlbum: async (name: string, emoji?: string) => {
      const trimmed = name.trim();
      if (!trimmed) return;
      await withBusy(async () => {
        const album = await api.createAlbum(trimmed, undefined, emoji);
        const ids =
          selectedList.length > 0
            ? selectedList
            : selectedId
              ? [selectedId]
              : [];
        if (ids.length > 0) await api.setAlbumItems(album.id, ids);
        await refreshMeta();
        setSelectedCollectionId(null);
        setExtraFilter({});
        setFilterBar((prev) => ({
          ...emptyFilterBarState(prev.sort, prev.sortDir),
          albumIds: [album.id],
        }));
        setNotification(successNotification(i18n.t("library:notification.albumCreated", { name: trimmed })));
      });
    },
    toggleAlbumOnSelection: async (albumId: number, add: boolean) => {
      if (selectedList.length === 0) return;
      await withBusy(async () => {
        const count = await runAlbumToggle(selectedList, albumId, add);
        await refreshMeta();
        if (selectedId !== null && selectedIds.has(selectedId)) {
          setDetail(await api.getAsset(selectedId));
        }
        if (count > 0) {
          setNotification(
            successNotification(
              add
                ? i18n.t("library:notification.albumAdded", { count })
                : i18n.t("library:notification.albumRemoved", { count }),
            ),
          );
        }
      });
    },
    createAlbumOnSelection: async (name: string) => {
      const trimmed = name.trim();
      if (!trimmed || selectedList.length === 0) return;
      let albumId = 0;
      await withBusy(async () => {
        albumId = await runAlbumCreate(selectedList, trimmed);
        await refreshMeta();
      });
      return albumId > 0 ? String(albumId) : undefined;
    },
    deleteAlbum: (id: number) => {
      if (stampReferencesAlbum(stampConfig, id)) {
        setNotification(
          errorNotification(i18n.t("library:notification.removeAlbumFromStamp")),
        );
        return;
      }
      requestConfirm(
        i18n.t("library:confirm.deleteAlbum.title"),
        i18n.t("library:confirm.deleteAlbum.message"),
        async () => {
          await api.deleteAlbum(id);
          await refreshMeta();
          setNotification(successNotification(i18n.t("library:notification.albumDeleted")));
        },
      );
    },
    selectAlbum: async (albumId: number) => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => ({
        ...emptyFilterBarState(prev.sort, prev.sortDir),
        albumIds: [albumId],
      }));
    },
    selectCollection: (sc: SmartCollection) => {
      setSelectedCollectionId(sc.id);
      setExtraFilter(assetFilterScope(sc.filter));
      setFilterBar((prev) =>
        filterBarFromAssetFilter(sc.filter, prev.sort, prev.sortDir),
      );
    },
    deleteCollection: (id: number) => {
      requestConfirm(
        i18n.t("library:confirm.deleteCollection.title"),
        i18n.t("library:confirm.deleteCollection.message"),
        async () => {
          await api.deleteSmartCollection(id);
          setSelectedCollectionId((prev) => (prev === id ? null : prev));
          await refreshMeta();
          setNotification(successNotification(i18n.t("library:notification.collectionDeleted")));
        },
      );
    },
    saveCollection: () => {
      setCollectionDialogOpen(true);
    },
    submitSaveCollection: async (name: string) => {
      const trimmed = name.trim();
      if (!trimmed) return;
      await withBusy(async () => {
        await api.saveSmartCollection(trimmed, filterRef.current);
        await refreshMeta();
        setCollectionDialogOpen(false);
        setNotification(successNotification(i18n.t("library:notification.collectionSaved", { name: trimmed })));
      });
    },
    softDeleteDuplicate: async (duplicateId: number) => {
      await withBusy(async () => {
        await api.softDeleteAssets([duplicateId]);
        if (selectedId !== null) {
          setDetail(await api.getAsset(selectedId));
        }
        await refreshGrid();
      });
    },
    relinkRoot: async (rootId: number) => {
      let path: string | null;
      try {
        path = await pickFolder();
      } catch (error) {
        setNotification(errorNotification(error));
        return;
      }
      if (!path) return;
      const preview = await api.previewRelink(rootId, path);
      requestConfirm(
        i18n.t("library:confirm.relinkRoot.title"),
        i18n.t("library:confirm.relinkRoot.message", {
          matched: preview.matched,
          sampled: preview.total_sampled,
          path: preview.new_path,
        }),
        async () => {
          await api.relinkRoot(rootId, path!);
          await api.startScan(rootId);
          await refreshAll();
          setNotification(successNotification(i18n.t("library:notification.rootRelinked")));
        },
      );
    },
    openGallery: () => {
      if (items.length === 0) {
        setNotification(infoNotification(i18n.t("library:notification.noSlideshowItems")));
        return;
      }
      const idx =
        selectedId !== null
          ? items.findIndex((i) => i.id === selectedId)
          : 0;
      const nextIndex = idx >= 0 ? idx : 0;
      const open = () => setGalleryIndex(nextIndex);
      if (document.startViewTransition) {
        document.startViewTransition(open);
      } else {
        open();
      }
    },
    closeInspector: () => {
      if (fullView) {
        setInspectorVisible(false);
        return;
      }
      setFullView(false);
      setSelectedIds(new Set());
      setSelectedId(null);
      setDetail(null);
      lastSelectedIndexRef.current = null;
    },
    closeDetail: () => {
      if (fullView) {
        setInspectorVisible(false);
        return;
      }
      setFullView(false);
      setSelectedIds(new Set());
      setSelectedId(null);
      setDetail(null);
      lastSelectedIndexRef.current = null;
    },
    clearSelection: () => {
      setSelectedIds(new Set());
      setSelectedId(null);
      setDetail(null);
      lastSelectedIndexRef.current = null;
    },
    loadMore,
    filterByTag: (tagId: number) => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => ({
        ...emptyFilterBarState(prev.sort, prev.sortDir),
        tagIds: [tagId],
      }));
    },
    createTag: async (name: string, parentId?: number, color?: string) => {
      await api.createTag(name, parentId, color);
      await refreshMeta();
      setNotification(
        successNotification(
          parentId
            ? i18n.t("library:notification.subtagCreated", { name })
            : i18n.t("library:notification.tagCreated", { name }),
        ),
      );
    },
    updateTag: async (id: number, name: string, color?: string) => {
      await withBusy(async () => {
        await api.updateTag(id, name, color);
        await refreshMeta();
        setNotification(successNotification(i18n.t("library:notification.tagUpdated", { name })));
      });
    },
    deleteTag: (id: number) => {
      if (stampReferencesTag(stampConfig, id)) {
        setNotification(
          errorNotification(i18n.t("library:notification.removeTagFromStamp")),
        );
        return;
      }
      requestConfirm(
        i18n.t("library:confirm.deleteTag.title"),
        i18n.t("library:confirm.deleteTag.message"),
        async () => {
          await api.deleteTag(id);
          setFilterBar((prev) => ({
            ...prev,
            tagIds: prev.tagIds.filter((tagId) => tagId !== id),
          }));
          await refreshMeta();
          setNotification(successNotification(i18n.t("library:notification.tagDeleted")));
        },
      );
    },
    updateAlbum: async (id: number, name: string, emoji?: string) => {
      await withBusy(async () => {
        await api.updateAlbum(id, name, emoji);
        await refreshMeta();
        setNotification(successNotification(i18n.t("library:notification.albumUpdated", { name })));
      });
    },
    viewTrash: () => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => ({
        ...emptyFilterBarState(prev.sort, prev.sortDir),
        deleteStatus: "deleted",
      }));
    },
    viewLibrary: () => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => emptyFilterBarState(prev.sort, prev.sortDir));
    },
    rebuildCatalog: () => {
      requestConfirm(
        i18n.t("library:confirm.rebuildCatalog.title"),
        i18n.t("library:confirm.rebuildCatalog.message"),
        async () => {
          await withBusy(async () => {
            await api.rebuildCatalog();
            setNotification(infoNotification(i18n.t("library:notification.catalogRebuildStarted")));
            await refreshAll();
          });
        },
      );
    },
    cancelScan: async () => {
      await api.cancelScan();
      setScanStatus("");
      setNotification(infoNotification(i18n.t("library:notification.scanCancelled")));
    },
    navigateGrid: (deltaCol: number, deltaRow: number) => {
      if (items.length === 0) return;
      const currentIdx =
        selectedId !== null
          ? items.findIndex((i) => i.id === selectedId)
          : -1;
      const nextIdx = navigateGridIndex(
        currentIdx,
        deltaCol,
        deltaRow,
        gridColumnCount,
        items.length,
      );
      const card = items[nextIdx];
      if (!card) return;
      lastSelectedIndexRef.current = nextIdx;
      setSelectedIds(new Set([card.id]));
      setSelectedId(card.id);
      void api.getAsset(card.id).then(setDetail);
    },
    navigateRelative: (delta: number) => {
      if (items.length === 0) return;
      const currentIdx =
        selectedId !== null
          ? items.findIndex((i) => i.id === selectedId)
          : -1;
      const nextIdx =
        currentIdx < 0
          ? delta > 0
            ? 0
            : items.length - 1
          : Math.max(0, Math.min(items.length - 1, currentIdx + delta));
      const card = items[nextIdx];
      if (!card) return;
      lastSelectedIndexRef.current = nextIdx;
      setSelectedIds(new Set([card.id]));
      setSelectedId(card.id);
      void api.getAsset(card.id).then(setDetail);
    },
    adjustGridSize: (delta: number) => {
      setGridColumnCount((prev) =>
        clampGridColumnCount(prev + delta * GRID_COLUMN_COUNT_STEP),
      );
    },
    toggleFullscreen: () => {
      if (fullView) {
        setFullView(false);
        return;
      }
      if (selectedId === null && items.length > 0) {
        const card = items[0];
        setSelectedIds(new Set([card.id]));
        setSelectedId(card.id);
        void api.getAsset(card.id).then(setDetail);
      }
      if (selectedId !== null || items.length > 0) {
        setFullView(true);
      }
    },
    openCompare: async () => {
      if (selectedList.length < 2) {
        setNotification(infoNotification(i18n.t("library:notification.selectTwoToCompare")));
        return;
      }
      setCompareIds(selectedList);
      setCompareOpen(true);
      const result = await api.queryAssets(
        { asset_ids: selectedList },
        sortParam,
        0,
        selectedList.length,
      );
      setCompareItems(result.items);
      const details = await Promise.all(selectedList.map((id) => api.getAsset(id)));
      setCompareDetails(
        Object.fromEntries(
          details.map((assetDetail) => [
            assetDetail.asset.id,
            {
              tag_ids: assetDetail.tag_ids,
              album_ids: assetDetail.album_ids,
            },
          ]),
        ),
      );
    },
    closeCompare: () => {
      setCompareOpen(false);
      setCompareIds([]);
      setCompareItems([]);
      setCompareDetails({});
    },
    rateAsset: async (id: number, rating: number) => {
      await api.updateAssetMeta(id, { rating });
      await refreshGrid();
    },
    toggleTagOnAsset: async (id: number, tagId: number, add: boolean) => {
      await withBusy(async () => {
        await runTagToggle([id], tagId, add);
        await refreshGrid();
        await refreshMeta();
        const assetDetail = await api.getAsset(id);
        setCompareDetails((prev) => ({
          ...prev,
          [id]: {
            tag_ids: assetDetail.tag_ids,
            album_ids: assetDetail.album_ids,
          },
        }));
      });
    },
    createTagOnAsset: async (id: number, tagName: string) => {
      const trimmed = tagName.trim();
      if (!trimmed) return;
      let tagId = 0;
      await withBusy(async () => {
        tagId = await runTagCreate([id], trimmed);
        await refreshMeta();
        await refreshGrid();
        const assetDetail = await api.getAsset(id);
        setCompareDetails((prev) => ({
          ...prev,
          [id]: {
            tag_ids: assetDetail.tag_ids,
            album_ids: assetDetail.album_ids,
          },
        }));
      });
      return tagId > 0 ? String(tagId) : undefined;
    },
    toggleAlbumOnAsset: async (id: number, albumId: number, add: boolean) => {
      await withBusy(async () => {
        await runAlbumToggle([id], albumId, add);
        await refreshMeta();
        const assetDetail = await api.getAsset(id);
        setCompareDetails((prev) => ({
          ...prev,
          [id]: {
            tag_ids: assetDetail.tag_ids,
            album_ids: assetDetail.album_ids,
          },
        }));
      });
    },
    createAlbumOnAsset: async (id: number, name: string) => {
      const trimmed = name.trim();
      if (!trimmed) return;
      let albumId = 0;
      await withBusy(async () => {
        albumId = await runAlbumCreate([id], trimmed);
        await refreshMeta();
        const assetDetail = await api.getAsset(id);
        setCompareDetails((prev) => ({
          ...prev,
          [id]: {
            tag_ids: assetDetail.tag_ids,
            album_ids: assetDetail.album_ids,
          },
        }));
      });
      return albumId > 0 ? String(albumId) : undefined;
    },
    disarmStamp,
    setStampRating,
    toggleStampTag: toggleStampTagInConfig,
    toggleStampAlbum: toggleStampAlbumInConfig,
    toggleStampOnTargets: async () => {
      if (!stampArmed || !isStampConfigValid(stampConfig)) {
        return;
      }
      const targets = resolveStampTargets({
        compareOpen,
        compareItemIds: compareItems.map((item) => item.id),
        fullView,
        selectedId,
        selectedList,
      });
      if (targets.length === 0) {
        setNotification(infoNotification(i18n.t("library:notification.selectItemsToStamp")));
        return;
      }
      await withBusy(async () => {
        const result = await runStampToggle(
          targets,
          stampConfig,
          async (id) => {
            const compareEntry = compareDetails[id];
            const card =
              compareItems.find((item) => item.id === id) ??
              items.find((item) => item.id === id);
            if (compareEntry && card) {
              return assetSnapshotFromParts(
                card.rating,
                compareEntry.tag_ids,
                compareEntry.album_ids,
              );
            }
            if (detail?.asset.id === id) {
              return assetSnapshotFromDetail(detail);
            }
            return assetSnapshotFromDetail(await api.getAsset(id));
          },
        );
        markStampResults(result.stampedIds, result.unstampedIds);
        await refreshGrid();
        await refreshMeta();
        if (selectedId !== null && targets.includes(selectedId)) {
          setDetail(await api.getAsset(selectedId));
        }
        if (compareOpen) {
          const refreshed = await Promise.all(
            targets.map((id) => api.getAsset(id)),
          );
          setCompareDetails((prev) => {
            const next = { ...prev };
            for (const assetDetail of refreshed) {
              next[assetDetail.asset.id] = {
                tag_ids: assetDetail.tag_ids,
                album_ids: assetDetail.album_ids,
              };
            }
            return next;
          });
        }
        if (result.unstamped > 0) {
          setNotification(
            successNotification(
              i18n.t("library:notification.unstamped", { count: result.unstamped }),
            ),
          );
        } else {
          setNotification(
            successNotification(
              i18n.t("library:notification.stamped", { count: result.applied }),
            ),
          );
        }
      });
    },
    deleteAsset: async (id: number) => {
      await api.softDeleteAssets([id]);
      setCompareIds((prev) => {
        const next = prev.filter((assetId) => assetId !== id);
        if (next.length < 2) {
          setCompareOpen(false);
          setCompareItems([]);
          setCompareDetails({});
          return [];
        }
        return next;
      });
      setCompareItems((prev) => prev.filter((item) => item.id !== id));
      setCompareDetails((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      setSelectedIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
      if (selectedId === id) {
        setSelectedId(null);
        setDetail(null);
      }
      await refreshGrid();
    },
  };
}
