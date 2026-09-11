import * as api from "../../../api/client";
import i18n from "../../../i18n";
import {
  infoNotification,
  successNotification,
  errorNotification,
} from "../../../lib/notification";
import { pickFolder } from "../../../lib/pickFolder";
import type { SmbConnectParams } from "../../../lib/libraryActions";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createRootsActions(deps: LibraryActionsDeps) {
  const {
    setNotification,
    refreshMeta,
    refreshAll,
    setSmbDialogOpen,
    requestConfirm,
    addRootAndScan,
  } = deps;

  return {
    addLocalRoot: () => addRootAndScan(api.addRoot),
    openSmbConnect: () => setSmbDialogOpen(true),
    addMountedSmbPath: async (path: string) => {
      setNotification(
        infoNotification(i18n.t("library:notification.addingSmbFolder")),
      );
      try {
        const root = await api.addSmbSource({
          mode: "mounted",
          path,
          poll_secs: 300,
        });
        await refreshMeta();
        setNotification(
          infoNotification(i18n.t("library:notification.addedSmbFolder")),
        );
        void api.startScan(root.id).catch((error) => {
          setNotification(errorNotification(error));
        });
      } catch (error) {
        setNotification(errorNotification(error));
      }
    },
    connectSmbShare: async (params: SmbConnectParams) => {
      setSmbDialogOpen(false);
      setNotification(
        infoNotification(i18n.t("library:notification.connectingSmbShare")),
      );
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
        setNotification(
          infoNotification(i18n.t("library:notification.smbShareConnected")),
        );
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
      setNotification(
        infoNotification(i18n.t("library:notification.scanning")),
      );
      void api.startScan(rootId).catch((error) => {
        setNotification(errorNotification(error));
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
          setNotification(
            successNotification(i18n.t("library:notification.rootRelinked")),
          );
        },
      );
    },
  };
}
