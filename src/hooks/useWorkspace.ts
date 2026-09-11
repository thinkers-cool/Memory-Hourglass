import { useCallback, useEffect, useState } from "react";
import * as api from "../api/client";
import i18n from "../i18n";
import { pickFolder } from "../lib/pickFolder";
import { useMessageSystem } from "./useMessageSystem";
import type { RecentWorkspace, WorkspaceInfo } from "../types";

export type WorkspacePhase = "loading" | "start" | "library";

export function useWorkspace() {
  const [phase, setPhase] = useState<WorkspacePhase>("loading");
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [recent, setRecent] = useState<RecentWorkspace[]>([]);
  const [busyMessage, setBusyMessage] = useState("");
  const {
    notification,
    setNotification,
    reportError,
    busy,
    withBusy,
    dismissToast,
  } = useMessageSystem();

  const refreshRecent = useCallback(async () => {
    const entries = await api.listRecentWorkspaces();
    setRecent(entries);
  }, []);

  const bootstrap = useCallback(async () => {
    setPhase("loading");
    setNotification(null);
    try {
      await refreshRecent();
      const opened = await api.tryOpenLastWorkspace();
      if (opened) {
        setWorkspace(opened);
        setPhase("library");
        return;
      }
      setWorkspace(null);
      setPhase("start");
    } catch (err) {
      setWorkspace(null);
      setPhase("start");
      reportError(err);
    }
  }, [refreshRecent, reportError, setNotification]);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  const openWorkspacePath = useCallback(
    async (path: string) => {
      setBusyMessage(i18n.t("common:busy.openingWorkspace"));
      setNotification(null);
      await withBusy(async () => {
        const opened = await api.openWorkspace(path);
        setWorkspace(opened);
        setPhase("library");
        await refreshRecent();
      });
      setBusyMessage("");
    },
    [refreshRecent, setNotification, withBusy],
  );

  const createWorkspaceAt = useCallback(
    async (path: string, readOnly = false) => {
      setBusyMessage(i18n.t("common:busy.creatingWorkspace"));
      setNotification(null);
      await withBusy(async () => {
        const created = await api.createWorkspace(path, readOnly);
        setWorkspace(created);
        setPhase("library");
        await refreshRecent();
      });
      setBusyMessage("");
    },
    [refreshRecent, setNotification, withBusy],
  );

  const pickAndOpenWorkspace = useCallback(async () => {
    let path: string | null;
    try {
      path = await pickFolder({
        title: i18n.t("common:folderPicker.openWorkspace"),
      });
    } catch (err) {
      reportError(err);
      return;
    }
    if (!path) return;
    await openWorkspacePath(path);
  }, [openWorkspacePath, reportError]);

  const pickAndCreateWorkspace = useCallback(
    async (readOnly = false) => {
      let path: string | null;
      try {
        path = await pickFolder({
          title: i18n.t("common:folderPicker.createWorkspace"),
          createDirectory: true,
        });
      } catch (err) {
        reportError(err);
        return;
      }
      if (!path) return;
      await createWorkspaceAt(path, readOnly);
    },
    [createWorkspaceAt, reportError],
  );

  const closeWorkspace = useCallback(async () => {
    setBusyMessage(i18n.t("common:busy.closingWorkspace"));
    setNotification(null);
    await withBusy(async () => {
      await api.closeWorkspace();
      setWorkspace(null);
      setPhase("start");
      await refreshRecent();
    });
    setBusyMessage("");
  }, [refreshRecent, setNotification, withBusy]);

  const removeRecent = useCallback(
    async (path: string) => {
      setBusyMessage(i18n.t("common:busy.removingWorkspace"));
      setNotification(null);
      await withBusy(async () => {
        await api.removeRecentWorkspace(path);
        await refreshRecent();
      });
      setBusyMessage("");
    },
    [refreshRecent, setNotification, withBusy],
  );

  return {
    phase,
    workspace,
    recent,
    busy,
    busyMessage,
    notification,
    setNotification,
    dismissToast,
    bootstrap,
    createWorkspaceAt,
    pickAndCreateWorkspace,
    openWorkspacePath,
    pickAndOpenWorkspace,
    closeWorkspace,
    removeRecent,
  };
}
