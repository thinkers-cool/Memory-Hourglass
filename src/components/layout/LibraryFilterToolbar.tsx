import { useMemo, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  BookmarkPlus,
  Presentation,
  Upload,
} from "lucide-react";
import { FilterChipBar } from "../shared/FilterChipBar";
import { StampButton } from "./StampButton";
import { GridSizeControl } from "../shared/GridSizeControl";
import { isStampConfigValid } from "../../lib/stamp";
import type { StampConfig } from "../../lib/stamp";
import { SortDropdown } from "../shared/SortDropdown";
import { ghostBtnClass } from "../../lib/buttonClass";
import { useToolbarCompact } from "../../hooks/useToolbarCompact";
import {
  applyLibraryFilterChange,
  buildLibraryFilterDefs,
  clearLibraryFilters,
  filterValuesFromBar,
  multiFilterValuesFromBar,
  removeLibraryFilter,
} from "../../lib/libraryFilters";
import type { FilterBarState } from "../../lib/libraryActions";
import type { Album, SortDir, SortMode, TagDto } from "../../types";

const toolbarActionButtonClass = (compact: boolean) =>
  compact
    ? ghostBtnClass("btn-sm btn-square h-8 min-h-0 w-8")
    : ghostBtnClass("btn-sm h-8 min-h-0 gap-1 text-xs font-normal");

export function LibraryFilterToolbar({
  busy: _busy,
  tags,
  albums,
  filterBar,
  setFilterBar,
  gridColumnCount,
  onGridColumnCountChange,
  onAdjustGridSize,
  onSaveCollection,
  showSaveCollection = true,
  onOpenSlideshow,
  onOpenExport,
  slideshowDisabled,
  stampConfig,
  stampArmed,
  onDisarmStamp,
  onStampRatingChange,
  onToggleStampTag,
  onToggleStampAlbum,
}: {
  busy: boolean;
  tags: TagDto[];
  albums: Album[];
  filterBar: FilterBarState;
  setFilterBar: React.Dispatch<React.SetStateAction<FilterBarState>>;
  gridColumnCount: number;
  onGridColumnCountChange: (count: number) => void;
  onAdjustGridSize: (delta: number) => void;
  onSaveCollection: () => void;
  showSaveCollection?: boolean;
  onOpenSlideshow: () => void;
  onOpenExport: () => void;
  slideshowDisabled: boolean;
  stampConfig: StampConfig;
  stampArmed: boolean;
  onDisarmStamp: () => void;
  onStampRatingChange: (rating: number | null) => void;
  onToggleStampTag: (tagId: number, add: boolean) => void;
  onToggleStampAlbum: (albumId: number, add: boolean) => void;
}) {
  const { t } = useTranslation(["library", "common"]);
  const toolbarRef = useRef<HTMLDivElement>(null);
  const compact = useToolbarCompact(toolbarRef);
  const filterDefs = useMemo(
    () => buildLibraryFilterDefs(tags, albums),
    [tags, albums],
  );
  const filterValues = useMemo(
    () => filterValuesFromBar(filterBar),
    [filterBar],
  );
  const multiFilterValues = useMemo(
    () => multiFilterValuesFromBar(filterBar),
    [filterBar],
  );

  const onFilterChange = useCallback(
    (id: string, value: string) => {
      applyLibraryFilterChange(id, value, setFilterBar);
    },
    [setFilterBar],
  );

  const onFilterRemove = useCallback(
    (id: string) => {
      removeLibraryFilter(id, setFilterBar);
    },
    [setFilterBar],
  );

  const onClearAll = useCallback(() => {
    clearLibraryFilters(setFilterBar, filterBar.sort, filterBar.sortDir);
  }, [setFilterBar, filterBar.sort, filterBar.sortDir]);

  const onDateChange = useCallback(
    (gte: string, lte: string) => {
      setFilterBar((prev) => ({
        ...prev,
        captureFrom: gte,
        captureTo: lte,
      }));
    },
    [setFilterBar],
  );

  const onSortChange = useCallback(
    (sort: SortMode) => {
      setFilterBar((prev) => ({ ...prev, sort }));
    },
    [setFilterBar],
  );

  const onSortDirChange = useCallback(
    (sortDir: SortDir) => {
      setFilterBar((prev) => ({ ...prev, sortDir }));
    },
    [setFilterBar],
  );

  return (
    <header className="surface-toolbar relative z-40 shrink-0 overflow-visible border-b">
      <div
        ref={toolbarRef}
        className="flex min-w-0 items-center gap-2 overflow-visible px-4 py-2"
      >
        <SortDropdown
          compact={compact}
          sort={filterBar.sort}
          sortDir={filterBar.sortDir}
          onSortChange={onSortChange}
          onSortDirChange={onSortDirChange}
        />

        <div className="h-5 w-px shrink-0 bg-divider" />

        <FilterChipBar
          compact={compact}
          filters={filterDefs}
          values={filterValues}
          multiValues={multiFilterValues}
          dateGte={filterBar.captureFrom}
          dateLte={filterBar.captureTo}
          onClearAll={onClearAll}
          onDateChange={onDateChange}
          onFilterChange={onFilterChange}
          onFilterRemove={onFilterRemove}
        />

        <div className="ml-auto flex shrink-0 items-center gap-1">
          {showSaveCollection ? (
            <button
              type="button"
              className={toolbarActionButtonClass(compact)}
              title={t("library:toolbar.saveAsCollection")}
              aria-label={t("common:aria.saveAsCollection")}
              onClick={onSaveCollection}
            >
              <BookmarkPlus className="h-3.5 w-3.5" />
              {!compact ? t("library:toolbar.saveAsCollection") : null}
            </button>
          ) : null}

          {showSaveCollection ? (
            <div className="mx-1 h-5 w-px bg-divider" />
          ) : null}

          <StampButton
            compact={compact}
            armed={stampArmed}
            configValid={isStampConfigValid(stampConfig)}
            config={stampConfig}
            tags={tags}
            albums={albums}
            onDisarm={onDisarmStamp}
            onRatingChange={onStampRatingChange}
            onToggleTag={onToggleStampTag}
            onToggleAlbum={onToggleStampAlbum}
          />

          <GridSizeControl
            compact={compact}
            value={gridColumnCount}
            onChange={onGridColumnCountChange}
            onAdjust={onAdjustGridSize}
          />

          <div className="mx-1 h-5 w-px bg-divider" />

          <button
            type="button"
            className={toolbarActionButtonClass(compact)}
            title={t("library:toolbar.slideshowTitle")}
            aria-label={t("common:aria.slideshow")}
            disabled={slideshowDisabled}
            onClick={onOpenSlideshow}
          >
            <Presentation className="h-3.5 w-3.5" />
            {!compact ? t("library:toolbar.slideshow") : null}
          </button>

          <button
            type="button"
            className={toolbarActionButtonClass(compact)}
            title={t("library:toolbar.export")}
            aria-label={t("common:action.export")}
            onClick={onOpenExport}
          >
            <Upload className="h-3.5 w-3.5" />
            {!compact ? t("library:toolbar.export") : null}
          </button>
        </div>
      </div>
    </header>
  );
}
