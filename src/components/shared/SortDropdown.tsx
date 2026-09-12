import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { usePopoverDismiss } from "../../hooks/usePopoverDismiss";
import {
  ArrowDown,
  ArrowUp,
  Calendar,
  FileText,
  FolderOpen,
  Star,
} from "lucide-react";
import { ghostBtnClass } from "../../lib/buttonClass";
import {
  MENU_ITEM_BUTTON_CLASS,
  MENU_POPOVER_CLASS,
} from "../../lib/formControlClass";
import { DEFAULT_SORT_DIR } from "../../lib/sortSettings";
import type { SortDir, SortMode } from "../../types";
import { IconTooltip } from "./Tooltip";

const SORT_OPTIONS: {
  labelKey: string;
  value: SortMode;
  icon: typeof Calendar;
}[] = [
  { labelKey: "sort.date", value: "date", icon: Calendar },
  { labelKey: "sort.name", value: "name", icon: FileText },
  { labelKey: "sort.rating", value: "rating", icon: Star },
  { labelKey: "sort.path", value: "path", icon: FolderOpen },
];

export function SortDropdown({
  sort,
  sortDir,
  onSortChange,
  onSortDirChange,
  compact = false,
}: {
  sort: SortMode;
  sortDir: SortDir;
  onSortChange: (sort: SortMode) => void;
  onSortDirChange: (dir: SortDir) => void;
  compact?: boolean;
}) {
  const { t } = useTranslation("common");
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const current =
    SORT_OPTIONS.find((opt) => opt.value === sort) ?? SORT_OPTIONS[0];
  const CurrentIcon = current.icon;
  const currentLabel = t(current.labelKey);
  const DirIcon = sortDir === "desc" ? ArrowDown : ArrowUp;
  const sortDirLabel =
    sortDir === "desc" ? t("sort.descending") : t("sort.ascending");
  const sortByLabel = t("sort.sortBy", { field: currentLabel });

  usePopoverDismiss({
    open,
    onClose: () => setOpen(false),
    containerRef: rootRef,
  });

  const toggleDir = () => {
    onSortDirChange(sortDir === "desc" ? "asc" : "desc");
  };

  const sortFieldButton = (
    <button
      type="button"
      className={`${ghostBtnClass("btn-sm join-item h-8 min-h-0 border border-control-border bg-control/65")} ${
        compact ? "btn-square w-8 px-0" : "gap-1.5 px-2.5 text-xs font-normal"
      }`}
      aria-label={sortByLabel}
      onClick={() => setOpen((prev) => !prev)}
    >
      <CurrentIcon className="h-3.5 w-3.5 text-content-faint" />
      {!compact ? (
        <span className="font-medium text-content-muted">{currentLabel}</span>
      ) : null}
    </button>
  );

  return (
    <div ref={rootRef} className="relative shrink-0">
      <div className="join">
        {compact ? (
          <IconTooltip tip={sortByLabel} placement="bottom">
            {sortFieldButton}
          </IconTooltip>
        ) : (
          sortFieldButton
        )}
        <IconTooltip tip={sortDirLabel} placement="bottom">
          <button
            type="button"
            className={`${ghostBtnClass("btn-sm join-item h-8 min-h-0 w-7 border border-control-border border-l-divider-subtle bg-control/65 px-0 text-content-muted")}`}
            aria-label={sortDirLabel}
            onClick={toggleDir}
          >
            <DirIcon className="h-3.5 w-3.5" />
          </button>
        </IconTooltip>
      </div>

      {open && (
        <div className="surface-popover absolute left-0 top-full z-[60] mt-1 min-w-[140px] p-1">
          <ul className={MENU_POPOVER_CLASS}>
            {SORT_OPTIONS.map((opt) => {
              const Icon = opt.icon;
              const selected = opt.value === sort;
              return (
                <li key={opt.value}>
                  <button
                    className={`${MENU_ITEM_BUTTON_CLASS} ${selected ? "active" : ""}`}
                    type="button"
                    onClick={() => {
                      onSortChange(opt.value);
                      onSortDirChange(DEFAULT_SORT_DIR[opt.value]);
                      setOpen(false);
                    }}
                  >
                    <Icon className="h-3.5 w-3.5 text-content-faint" />
                    {t(opt.labelKey)}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}
