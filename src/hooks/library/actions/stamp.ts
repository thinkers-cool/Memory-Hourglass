import * as api from "../../../api/client";
import i18n from "../../../i18n";
import {
  assetSnapshotFromDetail,
  assetSnapshotFromParts,
  isStampConfigValid,
  resolveStampTargets,
} from "../../../lib/stamp";
import { runStampToggle } from "../../../lib/stampMutations";
import {
  infoNotification,
  successNotification,
} from "../../../lib/notification";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createStampActions(deps: LibraryActionsDeps) {
  const {
    items,
    selectedId,
    selectedList,
    detail,
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
    setDetail,
    setCompareDetails,
    setNotification,
    refreshGrid,
    refreshMeta,
    withBusy,
  } = deps;

  return {
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
        fullView: deps.fullView,
        selectedId,
        selectedList,
      });
      if (targets.length === 0) {
        setNotification(
          infoNotification(i18n.t("library:notification.selectItemsToStamp")),
        );
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
              i18n.t("library:notification.unstamped", {
                count: result.unstamped,
              }),
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
  };
}
