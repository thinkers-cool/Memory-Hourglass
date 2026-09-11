import { X } from "lucide-react";
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
  roots,
  exportActive,
  exportProgress,
  notification,
  busy,
  onDismissAlert,
}: {
  total: number;
  selectedCount: number;
  scanStatus: string;
  roots: RootStats[];
  exportActive: boolean;
  exportProgress: ExportProgress;
  notification: Notification | null;
  busy: boolean;
  onDismissAlert?: () => void;
}) {
  const { t } = useTranslation(["library", "common"]);
  const display = resolveStatusDisplay({
    selectedCount,
    scanStatus,
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
          <div className="stat-title text-[10px]">{t("library:statusBar.library")}</div>
          <div className="stat-value text-xs">{total.toLocaleString()}</div>
        </div>
        <div className="stat min-w-0 flex-1 py-1 px-4 place-items-end">
          <div className="stat-title text-[10px]">{t("library:statusBar.status")}</div>
          <div
            className={`stat-value flex max-w-full items-center gap-2 truncate text-xs ${
              isError ? "text-error" : isSuccess ? "text-success" : ""
            }`}
          >
            {display.showSpinner && (
              <span className="loading loading-spinner loading-xs shrink-0 opacity-70" />
            )}
            <span className="truncate">{lineText}</span>
            {display.alert && onDismissAlert && (
              <button
                type="button"
                className="btn btn-ghost btn-xs btn-square h-6 min-h-0 w-6 shrink-0"
                onClick={onDismissAlert}
                aria-label={t("common:aria.close")}
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </div>
      </div>
    </footer>
  );
}
