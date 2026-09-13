import { RotateCcw, RotateCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ghostBtnClass } from "../../lib/buttonClass";
import { IconTooltip } from "./Tooltip";

const ICON_CLASS = "h-3.5 w-3.5";
const BTN_CLASS = ghostBtnClass("btn-xs btn-square h-7 min-h-0 w-7");

export function ImageViewerRotateControls({
  onRotateCounterClockwise,
  onRotateClockwise,
  disabled = false,
}: {
  onRotateCounterClockwise: () => void;
  onRotateClockwise: () => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation("common");

  return (
    <div className="flex shrink-0 items-center gap-0.5">
      <IconTooltip tip={t("aria.rotateCounterClockwise")} placement="top">
        <button
          type="button"
          className={BTN_CLASS}
          onClick={onRotateCounterClockwise}
          disabled={disabled}
          aria-label={t("aria.rotateCounterClockwise")}
        >
          <RotateCcw className={ICON_CLASS} />
        </button>
      </IconTooltip>
      <IconTooltip tip={t("aria.rotateClockwise")} placement="top">
        <button
          type="button"
          className={BTN_CLASS}
          onClick={onRotateClockwise}
          disabled={disabled}
          aria-label={t("aria.rotateClockwise")}
        >
          <RotateCw className={ICON_CLASS} />
        </button>
      </IconTooltip>
    </div>
  );
}
