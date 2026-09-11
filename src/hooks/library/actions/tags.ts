import * as api from "../../../api/client";
import i18n from "../../../i18n";
import { runTagCreate, runTagToggle } from "../../../lib/libraryMutations";
import { stampReferencesTag } from "../../../lib/stamp";
import {
  successNotification,
  errorNotification,
} from "../../../lib/notification";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createTagsActions(deps: LibraryActionsDeps) {
  const {
    selectedId,
    selectedIds,
    selectedList,
    stampConfig,
    setDetail,
    setFilterBar,
    setNotification,
    refreshMeta,
    refreshGrid,
    withBusy,
    requestConfirm,
  } = deps;

  return {
    toggleTagOnSelection: async (tagId: number, add: boolean) => {
      if (selectedList.length === 0) return;
      await withBusy(async () => {
        await runTagToggle(selectedList, tagId, add);
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
        setNotification(
          successNotification(
            i18n.t("library:notification.tagUpdated", { name }),
          ),
        );
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
          setNotification(
            successNotification(i18n.t("library:notification.tagDeleted")),
          );
        },
      );
    },
  };
}
