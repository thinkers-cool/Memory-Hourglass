import type { RootStats } from "../types";
import type { Notification } from "../types";
import i18n from "../i18n";
import type { ExportProgressState } from "./jobProgress";
import { isJobFinished } from "./jobProgress";
import { isErrorNotification, notificationText } from "./notification";

export type ExportProgress = ExportProgressState;

export type StatusProgress = {
  text: string;
};

export type StatusDisplay = {
  progress: StatusProgress | null;
  alert: Notification | null;
  flash: Notification | null;
  context: string;
  showSpinner: boolean;
};

export type StatusInput = {
  selectedCount: number;
  scanStatus: string;
  scanStatusByRoot?: Record<number, string>;
  focusScanRootId?: number | null;
  activeRootId?: number;
  roots: RootStats[];
  exportActive: boolean;
  exportProgress: ExportProgress;
  notification: Notification | null;
  busy: boolean;
  busyMessage?: string;
};

type ActiveScanEntry = { rootId: number; status: string };

function rootDisplayName(path: string): string {
  const normalized = path.replace(/[/\\]+$/, "");
  const segments = normalized.split(/[/\\]/).filter(Boolean);
  return segments[segments.length - 1] ?? path;
}

function listActiveScans(
  scanStatusByRoot: Record<number, string> | undefined,
): ActiveScanEntry[] {
  return Object.entries(scanStatusByRoot ?? {})
    .filter(([, status]) => isActiveScan(status))
    .map(([rootId, status]) => ({ rootId: Number(rootId), status }));
}

function progressForScanEntry(
  roots: RootStats[],
  entry: ActiveScanEntry,
): StatusProgress {
  const root = roots.find((item) => item.id === entry.rootId);
  const name = root ? rootDisplayName(root.path) : `#${entry.rootId}`;
  return {
    text: i18n.t("library:statusBar.scanningRoot", {
      root: name,
      progress: formatScanStatus(entry.status),
    }),
  };
}

export function isActiveScan(scanStatus: string): boolean {
  return (
    scanStatus !== "" &&
    !scanStatus.startsWith("done:") &&
    !scanStatus.startsWith("error:")
  );
}

export function formatScanStatus(scanStatus: string): string {
  if (scanStatus.startsWith("cataloging:")) {
    const [, counts] = scanStatus.split(": ");
    return i18n
      .t("library:statusBar.cataloging", { counts: counts ?? "" })
      .trim();
  }
  if (scanStatus.startsWith("indexing:")) {
    const [, counts] = scanStatus.split(": ");
    return i18n
      .t("library:statusBar.generatingPreviews", { counts: counts ?? "" })
      .trim();
  }
  if (scanStatus.startsWith("scanning:")) {
    const [, counts] = scanStatus.split(": ");
    return i18n
      .t("library:statusBar.scanning", { counts: counts ?? "" })
      .trim();
  }
  return scanStatus;
}

export function isProgressActive({
  busy,
  scanStatus,
  scanStatusByRoot,
  exportActive,
  exportProgress,
}: {
  busy: boolean;
  scanStatus: string;
  scanStatusByRoot?: Record<number, string>;
  exportActive: boolean;
  exportProgress: ExportProgress;
}): boolean {
  const hasActiveRootScans = Object.values(scanStatusByRoot ?? {}).some(
    isActiveScan,
  );
  return (
    busy ||
    isActiveScan(scanStatus) ||
    hasActiveRootScans ||
    (exportActive && exportProgress !== null)
  );
}

function formatSourceHealth(
  roots: RootStats[],
  activeRootId?: number,
): string | null {
  const scoped =
    activeRootId != null
      ? roots.filter((root) => root.id === activeRootId)
      : roots;
  const offline = scoped.filter((root) => root.status === "offline").length;
  const missing = scoped.reduce((sum, root) => sum + root.missing_count, 0);

  const parts: string[] = [];
  if (offline > 0) {
    parts.push(i18n.t("library:statusBar.offline", { count: offline }));
  }
  if (missing > 0) {
    parts.push(i18n.t("library:statusBar.missing", { count: missing }));
  }
  return parts.length > 0 ? parts.join(" · ") : null;
}

export function resolveProgressStatus(
  input: StatusInput,
): StatusProgress | null {
  if (input.busy && input.busyMessage?.trim()) {
    return { text: input.busyMessage.trim() };
  }

  if (input.exportActive && input.exportProgress) {
    if (
      input.exportProgress.file_name &&
      !isJobFinished(input.exportProgress.phase)
    ) {
      return {
        text: i18n.t("library:statusBar.exportFileProgress", {
          message: input.exportProgress.message,
          fileName: input.exportProgress.file_name,
          done: input.exportProgress.done,
          total: input.exportProgress.total,
        }),
      };
    }
    return {
      text: i18n.t("library:statusBar.exportProgress", {
        done: input.exportProgress.done,
        total: input.exportProgress.total,
      }),
    };
  }

  const activeScans = listActiveScans(input.scanStatusByRoot);
  if (activeScans.length === 1) {
    return progressForScanEntry(input.roots, activeScans[0]);
  }
  if (activeScans.length > 1) {
    const focusId = input.focusScanRootId;
    if (focusId != null) {
      const focused = activeScans.find((entry) => entry.rootId === focusId);
      if (focused) {
        return progressForScanEntry(input.roots, focused);
      }
    }
    return {
      text: i18n.t("library:statusBar.scanningSources", {
        count: activeScans.length,
      }),
    };
  }
  if (isActiveScan(input.scanStatus)) {
    return { text: formatScanStatus(input.scanStatus) };
  }

  if (input.busy) {
    return { text: i18n.t("library:statusBar.working") };
  }

  return null;
}

export function resolveContextStatus(input: StatusInput): string {
  if (input.selectedCount > 0) {
    return i18n.t("library:statusBar.selected", { count: input.selectedCount });
  }

  const sourceHealth = formatSourceHealth(input.roots, input.activeRootId);
  if (sourceHealth) {
    return sourceHealth;
  }

  return i18n.t("library:statusBar.ready");
}

export function resolveStatusDisplay(input: StatusInput): StatusDisplay {
  const progress = resolveProgressStatus(input);
  const alert =
    input.notification && isErrorNotification(input.notification)
      ? input.notification
      : null;
  const flash =
    input.notification && !isErrorNotification(input.notification)
      ? input.notification
      : null;
  const context = resolveContextStatus(input);
  const showSpinner = isProgressActive({
    busy: input.busy,
    scanStatus: input.scanStatus,
    scanStatusByRoot: input.scanStatusByRoot,
    exportActive: input.exportActive,
    exportProgress: input.exportProgress,
  });

  return { progress, alert, flash, context, showSpinner };
}

export function resolveStatusLine(display: StatusDisplay): string {
  if (display.progress) return display.progress.text;
  if (display.alert) return notificationText(display.alert);
  if (display.flash) return notificationText(display.flash);
  return display.context;
}

export function resolveSystemStatus(input: StatusInput): string {
  return resolveStatusLine(resolveStatusDisplay(input));
}
