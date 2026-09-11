import * as api from "../../../api/client";
import i18n from "../../../i18n";
import { emptyFilterBarState } from "../../../lib/libraryFilters";
import { runAlbumCreate, runAlbumToggle } from "../../../lib/libraryMutations";
import { stampReferencesAlbum } from "../../../lib/stamp";
import {
  successNotification,
  errorNotification,
} from "../../../lib/notification";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createAlbumsActions(deps: LibraryActionsDeps) {
  const {
    selectedId,
    selectedIds,
    selectedList,
    stampConfig,
    setDetail,
    setSelectedCollectionId,
    setExtraFilter,
    setFilterBar,
    setNotification,
    refreshMeta,
    withBusy,
    requestConfirm,
  } = deps;

  return {
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
        setNotification(
          successNotification(
            i18n.t("library:notification.albumCreated", { name: trimmed }),
          ),
        );
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
          errorNotification(
            i18n.t("library:notification.removeAlbumFromStamp"),
          ),
        );
        return;
      }
      requestConfirm(
        i18n.t("library:confirm.deleteAlbum.title"),
        i18n.t("library:confirm.deleteAlbum.message"),
        async () => {
          await api.deleteAlbum(id);
          await refreshMeta();
          setNotification(
            successNotification(i18n.t("library:notification.albumDeleted")),
          );
        },
      );
    },
    updateAlbum: async (id: number, name: string, emoji?: string) => {
      await withBusy(async () => {
        await api.updateAlbum(id, name, emoji);
        await refreshMeta();
        setNotification(
          successNotification(
            i18n.t("library:notification.albumUpdated", { name }),
          ),
        );
      });
    },
  };
}
