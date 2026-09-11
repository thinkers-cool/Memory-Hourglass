import { useCallback, useEffect, useState } from "react";
import * as api from "../../api/client";
import i18n from "../../i18n";
import {
  DEFAULT_EXPORT_OPTIONS,
  type ExportDialogState,
} from "../../lib/libraryActions";
import { isJobFinished, toExportProgress } from "../../lib/jobProgress";
import {
  errorNotification,
  infoNotification,
  successNotification,
} from "../../lib/notification";
import type { ExportOptions, Notification } from "../../types";

export function useLibraryExport(
  setNotification: React.Dispatch<React.SetStateAction<Notification | null>>,
) {
  const [exportDialog, setExportDialog] = useState<ExportDialogState>({
    open: false,
    assetIds: [],
    destination: "",
    options: DEFAULT_EXPORT_OPTIONS,
    jobId: null,
    progress: null,
  });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void api
      .onJobProgress((progress) => {
        setExportDialog((prev) => {
          if (prev.jobId === null || progress.job_id !== String(prev.jobId)) {
            return prev;
          }
          const finished = isJobFinished(progress.phase);
          return {
            ...prev,
            jobId: finished ? null : prev.jobId,
            progress: toExportProgress(progress),
          };
        });
        if (isJobFinished(progress.phase)) {
          setNotification(
            successNotification(
              i18n.t("library:statusBar.exportProgress", {
                done: progress.done,
                total: progress.total,
              }),
            ),
          );
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((error: unknown) => {
        setNotification(errorNotification(error));
      });
    return () => {
      unlisten?.();
    };
  }, [setNotification]);

  const openExport = useCallback((assetIds: number[]) => {
    if (assetIds.length === 0) return;
    setExportDialog({
      open: true,
      assetIds,
      destination: "",
      options: DEFAULT_EXPORT_OPTIONS,
      jobId: null,
      progress: null,
    });
  }, []);

  const runExport = useCallback(
    async (assetIds: number[], destination: string, options: ExportOptions) => {
      const jobId = Date.now();
      setExportDialog((prev) => ({
        ...prev,
        jobId,
        progress: {
          done: 0,
          total: assetIds.length,
          phase: "started",
          message: i18n.t("dialogs:export.started"),
        },
      }));
      await api.startExport(assetIds, destination, options, jobId);
    },
    [],
  );

  const closeExportDialog = useCallback(() => {
    setExportDialog((prev) => ({
      ...prev,
      open: false,
      jobId: null,
      progress: null,
    }));
  }, []);

  const updateExportDialog = useCallback(
    (patch: Partial<ExportDialogState>) => {
      setExportDialog((prev) => ({ ...prev, ...patch }));
    },
    [],
  );

  const startExportFromDialog = useCallback(async () => {
    const { assetIds, destination, options } = exportDialog;
    if (!destination || assetIds.length === 0) {
      setNotification(
        infoNotification(i18n.t("dialogs:export.selectDestination")),
      );
      return;
    }
    await runExport(assetIds, destination, options).catch((error: unknown) => {
      setNotification(errorNotification(error));
    });
  }, [exportDialog, runExport, setNotification]);

  return {
    exportDialog,
    openExport,
    runExport,
    closeExportDialog,
    updateExportDialog,
    startExportFromDialog,
  };
}
