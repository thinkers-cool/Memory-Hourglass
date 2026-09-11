import * as api from "../../../api/client";
import i18n from "../../../i18n";
import { infoNotification } from "../../../lib/notification";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createCatalogActions(deps: LibraryActionsDeps) {
  const {
    setScanStatus,
    setNotification,
    refreshAll,
    withBusy,
    requestConfirm,
  } = deps;

  return {
    rebuildCatalog: () => {
      requestConfirm(
        i18n.t("library:confirm.rebuildCatalog.title"),
        i18n.t("library:confirm.rebuildCatalog.message"),
        async () => {
          await withBusy(async () => {
            await api.rebuildCatalog();
            setNotification(
              infoNotification(
                i18n.t("library:notification.catalogRebuildStarted"),
              ),
            );
            await refreshAll();
          });
        },
      );
    },
    cancelScan: async () => {
      await api.cancelScan();
      setScanStatus("");
      setNotification(
        infoNotification(i18n.t("library:notification.scanCancelled")),
      );
    },
  };
}
