import { useTranslation } from "react-i18next";
import {
  resolveStatusDisplay,
  resolveStatusLine,
  type ExportProgress,
} from "../../lib/statusBar";
import type { Notification, RootStats } from "../../types";

export function StatusBar({
  total,
  selectedCount,
  scanStatus,
  scanStatusByRoot,
  focusScanRootId,
  activeRootId,
  roots,
  exportActive,
  exportProgress,
  notification,
  busy,
}: {
  total: number;
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
}) {
  const { t } = useTranslation(["library", "common"]);
  const display = resolveStatusDisplay({
    selectedCount,
    scanStatus,
    scanStatusByRoot,
    focusScanRootId,
    activeRootId,
    roots,
    exportActive,
    exportProgress,
    notification,
    busy,
  });
  const lineText = resolveStatusLine(display);
  const isError = display.alert !== null;
  const isSuccess = display.flash !== null;

  return (
    <footer className="surface-toolbar shrink-0 border-t">
      <div className="stats stats-horizontal w-full bg-transparent shadow-none">
        <div className="stat py-1 px-4">
          <div className="stat-title text-[10px]">
            {t("library:statusBar.library")}
          </div>
          <div className="stat-value text-xs">{total.toLocaleString()}</div>
        </div>
        <div className="stat min-w-0 flex-1 py-1 px-4 place-items-end">
          <div className="stat-title text-[10px]">
            {t("library:statusBar.status")}
          </div>
          <div
            className={`stat-value flex max-w-full items-center gap-2 truncate text-xs ${
              isError ? "text-error" : isSuccess ? "text-success" : ""
            }`}
          >
            {display.showSpinner && (
              <span className="loading loading-spinner loading-xs shrink-0 opacity-70" />
            )}
            <span className="truncate">{lineText}</span>
          </div>
        </div>
      </div>
    </footer>
  );
}
