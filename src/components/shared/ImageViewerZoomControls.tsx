import { ZoomIn, ZoomOut } from "lucide-react";
import { useTranslation } from "react-i18next";
import { formatImageViewerZoom } from "../../lib/imageViewerZoom";
import { ghostBtnClass } from "../../lib/buttonClass";

const ICON_CLASS = "h-3.5 w-3.5";

const BTN_CLASS = ghostBtnClass("btn-xs btn-square h-7 min-h-0 w-7");

export function ImageViewerZoomControls({
  zoom,
  onZoomIn,
  onZoomOut,
  onReset,
}: {
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onReset: () => void;
}) {
  const { t } = useTranslation("common");

  return (
    <div className="flex shrink-0 items-center gap-0.5">
      <button
        type="button"
        className={BTN_CLASS}
        onClick={onZoomOut}
        aria-label={t("aria.zoomOut")}
        title={t("aria.zoomOut")}
      >
        <ZoomOut className={ICON_CLASS} />
      </button>
      <button
        type="button"
        className="btn btn-ghost btn-xs h-7 min-h-0 px-2 text-xs tabular-nums text-content-muted hover:text-base-content"
        onClick={onReset}
        aria-label={t("aria.fitToView")}
        title={t("aria.fitToView")}
      >
        {formatImageViewerZoom(zoom)}
      </button>
      <button
        type="button"
        className={BTN_CLASS}
        onClick={onZoomIn}
        aria-label={t("aria.zoomIn")}
        title={t("aria.zoomIn")}
      >
        <ZoomIn className={ICON_CLASS} />
      </button>
    </div>
  );
}
