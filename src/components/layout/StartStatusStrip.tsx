import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  resolveStatusDisplay,
  resolveStatusLine,
} from "../../lib/statusBar";
import { isErrorNotification } from "../../lib/notification";
import type { Notification } from "../../types";

export function StartStatusStrip({
  busy,
  busyMessage,
  notification,
  onDismissAlert,
}: {
  busy: boolean;
  busyMessage: string;
  notification: Notification | null;
  onDismissAlert?: () => void;
}) {
  const { t } = useTranslation("common");
  const display = resolveStatusDisplay({
    selectedCount: 0,
    scanStatus: "",
    roots: [],
    exportActive: false,
    exportProgress: null,
    notification,
    busy,
    busyMessage,
  });
  const lineText = resolveStatusLine(display);
  const isError = display.alert !== null;
  const hasContent =
    display.progress !== null ||
    display.alert !== null ||
    display.flash !== null;

  if (!hasContent) return null;

  return (
    <footer
      className="surface-toolbar fixed inset-x-0 bottom-0 z-50 border-t"
      role={isError ? "alert" : "status"}
      aria-live={isError ? "assertive" : "polite"}
    >
      <div className="flex min-h-9 items-center gap-2 px-4 py-2 text-sm">
        {display.showSpinner && (
          <span className="loading loading-spinner loading-xs shrink-0 opacity-70" />
        )}
        <span
          className={`min-w-0 flex-1 truncate ${
            isError ? "text-error" : "text-base-content"
          }`}
        >
          {lineText}
        </span>
        {display.alert && onDismissAlert && (
          <button
            type="button"
            className="btn btn-ghost btn-xs btn-square h-6 min-h-0 w-6 shrink-0"
            onClick={onDismissAlert}
            aria-label={t("aria.close")}
          >
            <X className="h-3.5 w-3.5" />
          </button>
        )}
      </div>
    </footer>
  );
}
