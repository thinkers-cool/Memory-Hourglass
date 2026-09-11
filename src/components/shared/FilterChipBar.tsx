import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { createPortal } from "react-dom";
import { usePopoverDismiss } from "../../hooks/usePopoverDismiss";
import {
  Album,
  Calendar,
  Camera,
  Copy,
  Filter,
  MapPin,
  Plus,
  RefreshCw,
  Search,
  Star,
  Tag,
  Trash2,
  X,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { DateRangeFilter } from "./DateRangeFilter";
import { ghostBtnClass } from "../../lib/buttonClass";
import {
  INPUT_CONTROL_FULL_CLASS,
  MENU_ITEM_BUTTON_CLASS,
  MENU_POPOVER_CLASS,
} from "../../lib/formControlClass";
import { getDisplayValue, getMultiDisplayValue } from "../../lib/filterChipDisplay";

export interface FilterDef {
  id: string;
  label: string;
  type: "text" | "status" | "date";
  multi?: boolean;
  statusOptions?: string[];
  statusOptionLabels?: Record<string, string>;
  debounceMs?: number;
}

const FILTER_ICONS: Record<string, LucideIcon> = {
  rating: Star,
  sync: RefreshCw,
  delete: Trash2,
  camera: Camera,
  tag: Tag,
  album: Album,
  metadata: Search,
  capture: Calendar,
  gps: MapPin,
  duplicate: Copy,
};

interface FilterChipBarProps {
  filters: FilterDef[];
  values: Record<string, string>;
  multiValues?: Record<string, string[]>;
  onFilterChange: (id: string, value: string) => void;
  onFilterRemove: (id: string) => void;
  onClearAll: () => void;
  dateGte?: string;
  dateLte?: string;
  onDateChange?: (gte: string, lte: string) => void;
  compact?: boolean;
}

export function FilterChipBar({
  filters,
  values,
  multiValues = {},
  onFilterChange,
  onFilterRemove,
  onClearAll,
  dateGte = "",
  dateLte = "",
  onDateChange,
  compact = false,
}: FilterChipBarProps) {
  const { t } = useTranslation("common");
  const rootRef = useRef<HTMLDivElement>(null);
  const chipsRef = useRef<HTMLDivElement>(null);
  const expandedScrollWidthRef = useRef<number | null>(null);
  const [overflowIconOnly, setOverflowIconOnly] = useState(false);
  const iconOnly = compact || overflowIconOnly;
  const [menuOpen, setMenuOpen] = useState(false);
  const [openChip, setOpenChip] = useState<string | null>(null);
  const [pendingChipId, setPendingChipId] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const [prevFilters, setPrevFilters] = useState(filters);
  if (filters !== prevFilters) {
    setPrevFilters(filters);
    setOpenChip(null);
    setPendingChipId(null);
  }

  const filterHasValue = useCallback(
    (f: FilterDef) => {
      if (f.type === "date") return dateGte !== "" || dateLte !== "";
      if (f.multi) return (multiValues[f.id] ?? []).length > 0;
      return (values[f.id] ?? "") !== "";
    },
    [values, multiValues, dateGte, dateLte],
  );

  const activeFilters = useMemo(
    () =>
      filters.filter(
        (f) => filterHasValue(f) || pendingChipId === f.id,
      ),
    [filters, filterHasValue, pendingChipId],
  );
  const availableFilters = useMemo(
    () => filters.filter((f) => !filterHasValue(f) && pendingChipId !== f.id),
    [filters, filterHasValue, pendingChipId],
  );
  const hasActive = activeFilters.some((f) => filterHasValue(f));

  useLayoutEffect(() => {
    expandedScrollWidthRef.current = null;
    if (!compact) {
      setOverflowIconOnly(false);
    }
  }, [compact, activeFilters.length, hasActive, filters.length]);

  useLayoutEffect(() => {
    const element = chipsRef.current!;

    const measure = () => {
      setOverflowIconOnly((current) => {
        if (compact) {
          expandedScrollWidthRef.current = null;
          return false;
        }

        const width = element.clientWidth;
        const scrollWidth = element.scrollWidth;

        if (current) {
          if (
            expandedScrollWidthRef.current !== null &&
            width + 1 >= expandedScrollWidthRef.current
          ) {
            expandedScrollWidthRef.current = null;
            return false;
          }
          return true;
        }

        if (scrollWidth > width + 1) {
          expandedScrollWidthRef.current = scrollWidth;
          return true;
        }

        expandedScrollWidthRef.current = null;
        return false;
      });
    };

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [compact, iconOnly, activeFilters.length, hasActive]);

  useEffect(() => {
    if (!pendingChipId) return;
    const pending = filters.find((f) => f.id === pendingChipId);
    if (pending && filterHasValue(pending)) setPendingChipId(null);
  }, [pendingChipId, filters, filterHasValue]);

  useEffect(() => {
    const handler = () => setMenuOpen(true);
    window.addEventListener("memhg:focus-filter", handler);
    return () => window.removeEventListener("memhg:focus-filter", handler);
  }, []);

  usePopoverDismiss({
    open: menuOpen,
    onClose: () => setMenuOpen(false),
    containerRef: menuRef,
  });

  const handleChipDismiss = useCallback(
    (chipId: string) => {
      const pending = filters.find((f) => f.id === chipId);
      if (pending && !filterHasValue(pending)) {
        setPendingChipId(null);
        onFilterRemove(chipId);
      }
      setOpenChip(null);
    },
    [filters, filterHasValue, onFilterRemove],
  );

  const handleAddFilter = useCallback(
    (f: FilterDef) => {
      setMenuOpen(false);
      if (f.type === "status") {
        if (f.multi) {
          setPendingChipId(f.id);
        } else {
          onFilterChange(f.id, f.statusOptions?.[0] ?? "");
        }
      } else {
        if (f.type === "text") onFilterChange(f.id, "");
        setPendingChipId(f.id);
      }
      setTimeout(() => setOpenChip(f.id), 0);
    },
    [onFilterChange],
  );

  const handleRemoveFilter = useCallback(
    (f: FilterDef) => {
      if (pendingChipId === f.id) setPendingChipId(null);
      if (f.type === "date" && onDateChange) {
        onDateChange("", "");
      }
      onFilterRemove(f.id);
      if (openChip === f.id) setOpenChip(null);
    },
    [onFilterRemove, onDateChange, openChip, pendingChipId],
  );

  return (
    <div
      ref={rootRef}
      className="relative z-0 flex min-w-0 flex-1 items-center gap-1"
    >
      <div ref={menuRef} className="relative shrink-0">
        <button
          className={`btn btn-ghost btn-interactive btn-sm border border-dashed border-interactive-border hover:border-interactive-selected-border hover:bg-interactive-selected-subtle h-8 min-h-0 ${
            iconOnly ? "btn-square w-8 px-0" : "gap-1 text-xs font-normal"
          }`}
          type="button"
          title={t("aria.addFilter")}
          aria-label={t("aria.addFilter")}
          onClick={() => setMenuOpen(!menuOpen)}
        >
          <Filter className="h-3.5 w-3.5" />
          {!iconOnly ? (
            <>
              {t("filter.add")}
              {availableFilters.length > 0 ? <Plus className="h-3 w-3" /> : null}
            </>
          ) : null}
        </button>

        {menuOpen && availableFilters.length > 0 && (
          <div className="surface-popover absolute left-0 top-full z-[60] mt-1 min-w-[160px] p-1">
            <ul className={MENU_POPOVER_CLASS}>
              {availableFilters.map((f) => {
                const Icon = FILTER_ICONS[f.id];
                return (
                  <li key={f.id}>
                    <button
                      className={MENU_ITEM_BUTTON_CLASS}
                      type="button"
                      onClick={() => handleAddFilter(f)}
                    >
                      {Icon && <Icon className="h-3.5 w-3.5 text-content-faint" />}
                      {f.label}
                    </button>
                  </li>
                );
              })}
            </ul>
          </div>
        )}
      </div>

      <div
        ref={chipsRef}
        className="flex min-w-0 flex-1 items-center gap-1 overflow-hidden"
      >
      {activeFilters.map((f) => (
        <div key={f.id} className="relative shrink-0">
          <Chip
            iconOnly={iconOnly}
            dateGte={dateGte}
            dateLte={dateLte}
            filter={f}
            isOpen={openChip === f.id}
            value={values[f.id] ?? ""}
            selectedValues={multiValues[f.id] ?? []}
            onChange={(v) => onFilterChange(f.id, v)}
            onDateChange={onDateChange}
            onRemove={() => handleRemoveFilter(f)}
            onClose={() => handleChipDismiss(f.id)}
            onToggle={() => {
              if (openChip === f.id) {
                handleChipDismiss(f.id);
              } else {
                setOpenChip(f.id);
              }
            }}
          />
        </div>
      ))}

      {hasActive ? (
        <button
          className={`btn btn-ghost btn-interactive btn-sm h-8 min-h-0 shrink-0 text-content-faint hover:text-error ${
            iconOnly ? "btn-square w-8 px-0" : "gap-1 text-xs font-normal"
          }`}
          type="button"
          title={t("aria.clearAllFilters")}
          aria-label={t("aria.clearAllFilters")}
          onClick={() => {
            setPendingChipId(null);
            onClearAll();
          }}
        >
          <X className="h-3.5 w-3.5" />
          {!iconOnly ? t("filter.clearAll") : null}
        </button>
      ) : null}
      </div>
    </div>
  );
}

interface ChipProps {
  filter: FilterDef;
  value: string;
  selectedValues: string[];
  dateGte: string;
  dateLte: string;
  isOpen: boolean;
  iconOnly?: boolean;
  onToggle: () => void;
  onClose: () => void;
  onChange: (value: string) => void;
  onDateChange?: (gte: string, lte: string) => void;
  onRemove: () => void;
}

function Chip({
  filter,
  value,
  selectedValues,
  dateGte,
  dateLte,
  isOpen,
  iconOnly = false,
  onToggle,
  onClose,
  onChange,
  onDateChange,
  onRemove,
}: ChipProps) {
  const { t } = useTranslation("common");
  const inputRef = useRef<HTMLInputElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const chipRef = useRef<HTMLDivElement>(null);
  const FilterIcon = FILTER_ICONS[filter.id];
  const [draft, setDraft] = useState(value);
  const [popoverPosition, setPopoverPosition] = useState<{
    top: number;
    left: number;
  } | null>(null);
  const debounceMs = filter.debounceMs ?? 0;

  useLayoutEffect(() => {
    if (!isOpen || !chipRef.current) {
      setPopoverPosition(null);
      return;
    }

    const updatePosition = () => {
      const rect = chipRef.current!.getBoundingClientRect();
      setPopoverPosition({ top: rect.bottom + 4, left: rect.left });
    };

    updatePosition();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [isOpen]);

  usePopoverDismiss({
    open: isOpen,
    onClose,
    containerRef: popoverRef,
    anchorRef: chipRef,
  });

  useEffect(() => {
    setDraft(value);
  }, [value]);

  useEffect(() => {
    if (!debounceMs) return;
    const timer = window.setTimeout(() => {
      if (draft !== value) onChange(draft);
    }, debounceMs);
    return () => window.clearTimeout(timer);
  }, [draft, debounceMs, onChange, value]);

  useEffect(() => {
    if (isOpen && filter.type === "text" && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen, filter.type, popoverPosition]);

  const displayValue = filter.multi
    ? getMultiDisplayValue(filter, selectedValues)
    : getDisplayValue(filter, value, dateGte, dateLte);
  const inputValue = debounceMs ? draft : value;

  const handleTextChange = (next: string) => {
    if (debounceMs) {
      setDraft(next);
      return;
    }
    onChange(next);
  };

  const chipTitle = displayValue
    ? t("aria.filterChip", { filter: filter.label, value: displayValue })
    : t("aria.filterChipLabel", { filter: filter.label });

  return (
    <>
      <div
        ref={chipRef}
        className="inline-flex h-8 shrink-0 cursor-pointer items-center gap-1 rounded-md border border-solid border-control-border bg-interactive-selected-subtle text-xs transition-colors hover:border-interactive-selected-border"
      >
        <button
          className={`${ghostBtnClass("btn-xs h-full max-w-full border-0 bg-transparent shadow-none")} ${
            iconOnly ? "gap-1 pl-2" : "gap-1.5 pl-2.5"
          }`}
          type="button"
          title={iconOnly ? chipTitle : undefined}
          aria-label={iconOnly ? chipTitle : undefined}
          onClick={onToggle}
        >
          {FilterIcon ? (
            <FilterIcon className="h-3.5 w-3.5 shrink-0 text-content-faint" />
          ) : null}
          {!iconOnly ? (
            <span className="font-medium text-content-faint">{filter.label}</span>
          ) : null}
          {displayValue ? (
            <>
              {!iconOnly ? <span className="text-content-quaternary">:</span> : null}
              <span
                className={`font-medium text-base-content ${
                  iconOnly ? "max-w-[4.5rem] truncate" : ""
                }`}
              >
                {displayValue}
              </span>
            </>
          ) : null}
        </button>
        <button
          className={`${ghostBtnClass("btn-xs h-full min-h-0 rounded-none rounded-r-md border-0 border-l border-divider-subtle px-1.5 text-danger-text shadow-none hover:bg-danger-subtle hover:text-danger")}`}
          type="button"
          title={t("aria.removeFilter", { filter: filter.label })}
          aria-label={t("aria.removeFilter", { filter: filter.label })}
          onClick={(event) => {
            event.stopPropagation();
            onRemove();
          }}
        >
          <X className="h-3 w-3" />
        </button>
      </div>

      {isOpen && popoverPosition
        ? createPortal(
            <div
              ref={popoverRef}
              className="surface-popover fixed z-[60] p-2"
              style={{
                top: popoverPosition.top,
                left: popoverPosition.left,
              }}
              onClick={(event) => event.stopPropagation()}
            >
              {filter.type === "text" && (
                <input
                  ref={inputRef}
                  className={`${INPUT_CONTROL_FULL_CLASS} w-48 text-sm`}
                  placeholder={t("filter.filterBy", { label: filter.label })}
                  type="text"
                  value={inputValue}
                  onChange={(e) => handleTextChange(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") onClose();
                    if (e.key === "Escape" && debounceMs) {
                      setDraft("");
                      onChange("");
                      onClose();
                    }
                  }}
                />
              )}

              {filter.type === "status" && (
                <ul className={`${MENU_POPOVER_CLASS} min-w-[120px]`}>
                  {(filter.statusOptions ?? []).map((opt) => {
                    const active = filter.multi
                      ? selectedValues.includes(opt)
                      : value === opt;
                    return (
                      <li key={opt}>
                        <button
                          className={`${MENU_ITEM_BUTTON_CLASS} ${active ? "active" : ""}`}
                          type="button"
                          onClick={() => {
                            onChange(opt);
                            if (!filter.multi) onClose();
                          }}
                        >
                          {filter.statusOptionLabels?.[opt] ?? opt}
                        </button>
                      </li>
                    );
                  })}
                </ul>
              )}

              {filter.type === "date" && onDateChange ? (
                <DateRangeFilter
                  dateGte={dateGte}
                  dateLte={dateLte}
                  size="xs"
                  onChange={onDateChange}
                />
              ) : null}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
