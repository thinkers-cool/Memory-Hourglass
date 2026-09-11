import * as api from "../../../api/client";
import i18n from "../../../i18n";
import { successNotification } from "../../../lib/notification";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createDeleteActions(deps: LibraryActionsDeps) {
  const {
    selectedId,
    selectedList,
    purgeTargetIds,
    readOnly,
    setSelectedId,
    setSelectedIds,
    setDetail,
    setPurgeDialogOpen,
    setPurgeTargetIds,
    setCompareOpen,
    setCompareIds,
    setCompareItems,
    setCompareDetails,
    setNotification,
    refreshGrid,
    withBusy,
  } = deps;

  return {
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
        selectedList.length > 0 ? selectedList : selectedId ? [selectedId] : [];
      if (ids.length === 0) return;
      const count = await api.restoreAssets(ids);
      setSelectedIds(new Set());
      setSelectedId(null);
      setDetail(null);
      await refreshGrid();
      setNotification(
        successNotification(i18n.t("library:notification.restored", { count })),
      );
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
