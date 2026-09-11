import * as api from "../../../api/client";
import {
  toggleSelection,
  applyRangeSelection,
  resolveSelectionAnchor,
} from "../../../lib/selection";
import { navigateGridIndex } from "../../../lib/gridNavigation";
import {
  clampGridColumnCount,
  GRID_COLUMN_COUNT_STEP,
} from "../../../lib/gridSettings";
import type { AssetCard } from "../../../types";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createSelectionActions(deps: LibraryActionsDeps) {
  const {
    items,
    selectedId,
    selectedIds,
    selectedList,
    gridColumnCount,
    fullView,
    lastSelectedIndexRef,
    setSelectedId,
    setSelectedIds,
    setDetail,
    setFullView,
    setInspectorVisible,
    setGridColumnCount,
    setTagMenuOpen,
    setAlbumMenuOpen,
    refreshGrid,
    loadMore,
    openExport,
  } = deps;

  return {
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
    navigateGrid: (deltaCol: number, deltaRow: number) => {
      if (items.length === 0) return;
      const currentIdx =
        selectedId !== null ? items.findIndex((i) => i.id === selectedId) : -1;
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
        selectedId !== null ? items.findIndex((i) => i.id === selectedId) : -1;
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
  };
}
