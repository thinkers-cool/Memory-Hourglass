import { useCallback, useEffect, useMemo, useState } from "react";
import * as api from "../api/client";
import {
  loadGridColumnCount,
  saveGridColumnCount,
} from "../lib/gridSettings";
import { infoNotification, errorNotification } from "../lib/notification";
import { pickFolder } from "../lib/pickFolder";
import { createLibraryActions } from "./library/createLibraryActions";
import { useLibraryConfirm } from "./library/useLibraryConfirm";
import { useLibraryExport } from "./library/useLibraryExport";
import { useMessageSystem } from "./useMessageSystem";
import { useLibraryQuery } from "./library/useLibraryQuery";
import { useLibrarySelection } from "./library/useLibrarySelection";
import { useStamp } from "./library/useStamp";

export function useLibrary(readOnly = false) {
  const {
    notification,
    setNotification,
    busy,
    withBusy,
    dismissToast,
    undoActivity: undoActivityBase,
  } = useMessageSystem();
  const { confirmDialog, requestConfirm, closeConfirmDialog } =
    useLibraryConfirm();
  const selection = useLibrarySelection();

  const query = useLibraryQuery(
    selection.selectedIdRef,
    selection.setDetail,
    setNotification,
  );

  useEffect(() => {
    if (selection.selectedId === null) return;
    if (!query.items.some((item) => item.id === selection.selectedId)) {
      selection.setSelectedId(null);
      selection.setSelectedIds(new Set());
      selection.setDetail(null);
      selection.lastSelectedIndexRef.current = null;
    }
  }, [
    query.items,
    selection.selectedId,
    selection.setSelectedId,
    selection.setSelectedIds,
    selection.setDetail,
    selection.lastSelectedIndexRef,
  ]);

  const [galleryIndex, setGalleryIndex] = useState<number | null>(null);
  const [fullView, setFullView] = useState(false);
  const [inspectorVisible, setInspectorVisible] = useState(true);
  const [gridColumnCount, setGridColumnCount] = useState(loadGridColumnCount);
  const [smbDialogOpen, setSmbDialogOpen] = useState(false);
  const [collectionDialogOpen, setCollectionDialogOpen] = useState(false);
  const [tagMenuOpen, setTagMenuOpen] = useState(false);
  const [albumMenuOpen, setAlbumMenuOpen] = useState(false);
  const [purgeDialogOpen, setPurgeDialogOpen] = useState(false);
  const [purgeTargetIds, setPurgeTargetIds] = useState<number[]>([]);

  const stamp = useStamp(query.tags, query.albums);

  const {
    exportDialog,
    openExport,
    closeExportDialog,
    updateExportDialog,
    startExportFromDialog,
  } = useLibraryExport(setNotification);

  useEffect(() => {
    saveGridColumnCount(gridColumnCount);
  }, [gridColumnCount]);

  const undoActivity = useCallback(
    async (activityId: number) => {
      await undoActivityBase(activityId);
      await query.refreshAll();
    },
    [undoActivityBase, query.refreshAll],
  );

  const addRootAndScan = useCallback(
    async (add: (path: string) => Promise<{ id: number; path: string }>) => {
      let path: string | null;
      try {
        path = await pickFolder();
      } catch (error) {
        setNotification(errorNotification(error));
        return;
      }
      if (!path) return;

      await withBusy(async () => {
        const root = await add(path);
        await query.refreshMeta();
        setNotification(infoNotification("Added folder — scanning…"));
        void api.startScan(root.id).catch((error) => {
          setNotification(errorNotification(error));
        });
      });
    },
    [query.refreshMeta, setNotification, withBusy],
  );

  const actions = useMemo(
    () =>
      createLibraryActions({
        items: query.items,
        selectedId: selection.selectedId,
        selectedIds: selection.selectedIds,
        selectedList: selection.selectedList,
        detail: selection.detail,
        gridColumnCount,
        fullView,
        compareOpen: selection.compareOpen,
        compareItems: selection.compareItems,
        compareDetails: selection.compareDetails,
        stampConfig: stamp.config,
        stampArmed: stamp.armed,
        disarmStamp: stamp.disarm,
        setStampRating: stamp.setStampRating,
        toggleStampTagInConfig: stamp.toggleStampTag,
        toggleStampAlbumInConfig: stamp.toggleStampAlbum,
        markStampResults: stamp.markStampResults,
        purgeTargetIds,
        readOnly,
        filterRef: query.filterRef,
        sortParam: query.sortParam,
        lastSelectedIndexRef: selection.lastSelectedIndexRef,
        setSelectedId: selection.setSelectedId,
        setSelectedIds: selection.setSelectedIds,
        setDetail: selection.setDetail,
        setExtraFilter: query.setExtraFilter,
        setFilterBar: query.setFilterBar,
        setSelectedCollectionId: query.setSelectedCollectionId,
        setFullView,
        setInspectorVisible,
        setGalleryIndex,
        setCompareOpen: selection.setCompareOpen,
        setCompareIds: selection.setCompareIds,
        setCompareItems: selection.setCompareItems,
        setCompareDetails: selection.setCompareDetails,
        setSmbDialogOpen,
        setCollectionDialogOpen,
        setTagMenuOpen,
        setAlbumMenuOpen,
        setPurgeDialogOpen,
        setPurgeTargetIds,
        setGridColumnCount,
        setScanStatus: query.setScanStatus,
        setNotification,
        refreshMeta: query.refreshMeta,
        refreshGrid: query.refreshGrid,
        refreshAll: query.refreshAll,
        loadMore: query.loadMore,
        withBusy,
        openExport,
        requestConfirm,
        addRootAndScan,
      }),
    [
      addRootAndScan,
      fullView,
      gridColumnCount,
      inspectorVisible,
      stamp.armed,
      stamp.config,
      stamp.markStampResults,
      stamp.disarm,
      stamp.setStampRating,
      stamp.toggleStampAlbum,
      stamp.toggleStampTag,
      selection.compareDetails,
      selection.compareItems,
      selection.compareOpen,
      openExport,
      purgeTargetIds,
      readOnly,
      query.filterRef,
      query.items,
      query.loadMore,
      query.refreshAll,
      query.refreshGrid,
      query.refreshMeta,
      query.setExtraFilter,
      query.setFilterBar,
      query.setScanStatus,
      query.setSelectedCollectionId,
      query.sortParam,
      requestConfirm,
      selection.detail,
      selection.lastSelectedIndexRef,
      selection.selectedId,
      selection.selectedIds,
      selection.selectedList,
      selection.setCompareIds,
      selection.setCompareItems,
      selection.setCompareDetails,
      selection.setCompareOpen,
      selection.setDetail,
      selection.setSelectedId,
      selection.setSelectedIds,
      setNotification,
      withBusy,
    ],
  );

  const closeCollectionDialog = useCallback(() => {
    setCollectionDialogOpen(false);
  }, []);

  return {
    roots: query.roots,
    albums: query.albums,
    collections: query.collections,
    tags: query.tags,
    deletedCount: query.deletedCount,
    items: query.items,
    total: query.total,
    detail: selection.detail,
    busy,
    notification,
    scanStatus: query.scanStatus,
    galleryIndex,
    setGalleryIndex,
    fullView,
    inspectorVisible,
    gridColumnCount,
    setGridColumnCount,
    compareOpen: selection.compareOpen,
    compareItems: selection.compareItems,
    compareDetails: selection.compareDetails,
    smbDialogOpen,
    setSmbDialogOpen,
    collectionDialogOpen,
    closeCollectionDialog,
    tagMenuOpen,
    setTagMenuOpen,
    albumMenuOpen,
    setAlbumMenuOpen,
    purgeDialogOpen,
    purgeTargetIds,
    closePurgeDialog: () => {
      setPurgeDialogOpen(false);
      setPurgeTargetIds([]);
    },
    loadingMore: query.loadingMore,
    hasMore: query.hasMore,
    filterBar: query.filterBar,
    extraFilter: query.extraFilter,
    selectedCollectionId: query.selectedCollectionId,
    setFilterBar: query.setFilterBar,
    selectedId: selection.selectedId,
    selectedIds: selection.selectedIds,
    actions,
    exportDialog,
    closeExportDialog,
    updateExportDialog,
    startExportFromDialog,
    confirmDialog,
    closeConfirmDialog,
    stampConfig: stamp.config,
    stampArmed: stamp.armed,
    stampMatchedIds: stamp.matchedIds,
    dismissToast,
    undoActivity,
  };
}
