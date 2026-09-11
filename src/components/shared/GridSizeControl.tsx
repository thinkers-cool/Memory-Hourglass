import { useEffect, useRef, useState } from "react";
import { LayoutGrid, Minus, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { RANGE_CONTROL_CLASS } from "../../lib/formControlClass";
import {
  GRID_COLUMN_COUNT_MAX,
  GRID_COLUMN_COUNT_MIN,
  GRID_COLUMN_COUNT_STEP,
  clampGridColumnCount,
} from "../../lib/gridSettings";

export function GridSizeControl({
  value,
  onChange,
  onAdjust,
  compact = false,
}: {
  value: number;
  onChange: (columnCount: number) => void;
  onAdjust: (delta: number) => void;
  compact?: boolean;
}) {
  const { t } = useTranslation("common");
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const columnsLabel = t("aria.gridColumnsValue", { count: value });

  useEffect(() => {
    if (!open) return;
    const onMouseDown = (event: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onMouseDown);
    return () => document.removeEventListener("mousedown", onMouseDown);
  }, [open]);

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        className={`btn btn-ghost btn-interactive btn-xs h-8 min-h-0 ${
          compact ? "btn-square w-8 px-0" : "gap-1 px-2 text-xs font-normal"
        }`}
        title={columnsLabel}
        aria-label={columnsLabel}
        onClick={() => setOpen((prev) => !prev)}
      >
        <LayoutGrid className="h-4 w-4" />
        {!compact ? t("grid.label") : null}
      </button>

      {open && (
        <div
          className="surface-popover absolute right-0 top-full z-[60] mt-1 p-2"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-center gap-1.5">
            <button
              type="button"
              className="btn btn-ghost btn-interactive btn-xs btn-square min-h-0 h-7 w-7"
              disabled={value <= GRID_COLUMN_COUNT_MIN}
              onClick={() => onAdjust(-GRID_COLUMN_COUNT_STEP)}
            >
              <Minus className="h-3.5 w-3.5" />
            </button>
            <input
              type="range"
              min={GRID_COLUMN_COUNT_MIN}
              max={GRID_COLUMN_COUNT_MAX}
              step={GRID_COLUMN_COUNT_STEP}
              value={value}
              onChange={(e) =>
                onChange(clampGridColumnCount(Number(e.target.value)))
              }
              className={`${RANGE_CONTROL_CLASS} w-24`}
              aria-label={t("aria.gridColumns")}
            />
            <button
              type="button"
              className="btn btn-ghost btn-interactive btn-xs btn-square min-h-0 h-7 w-7"
              disabled={value >= GRID_COLUMN_COUNT_MAX}
              onClick={() => onAdjust(GRID_COLUMN_COUNT_STEP)}
            >
              <Plus className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
