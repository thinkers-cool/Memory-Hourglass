import * as api from "../../../api/client";
import i18n from "../../../i18n";
import {
  assetFilterScope,
  filterBarFromAssetFilter,
} from "../../../lib/libraryFilters";
import { successNotification } from "../../../lib/notification";
import type { SmartCollection } from "../../../types";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createCollectionsActions(deps: LibraryActionsDeps) {
  const {
    filterRef,
    setSelectedCollectionId,
    setExtraFilter,
    setFilterBar,
    setCollectionDialogOpen,
    setNotification,
    refreshMeta,
    withBusy,
    requestConfirm,
  } = deps;

  return {
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
          setNotification(
            successNotification(
              i18n.t("library:notification.collectionDeleted"),
            ),
          );
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
        setNotification(
          successNotification(
            i18n.t("library:notification.collectionSaved", { name: trimmed }),
          ),
        );
      });
    },
  };
}
