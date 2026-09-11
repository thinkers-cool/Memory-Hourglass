import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { RatingRow } from "../RatingRow";
import { SelectionPickerPopover } from "../shared/SelectionPickerPopover";
import type { Album, TagDto } from "../../types";
import type { FloatingBarMode } from "../../lib/floatingBarMode";
import {
  buildAlbumPickerItems,
  buildTagPickerItems,
  ToolbarDivider,
} from "../../lib/pickerItems";

function ShortcutHint({ children }: { children: string }) {
  return (
    <span className="ml-1 text-[10px] font-normal uppercase tracking-wide text-content-faint">
      {children}
    </span>
  );
}

export function FloatingSelectionBar({
  mode,
  count,
  tags,
  albums,
  busy,
  tagMenuOpen,
  onTagMenuOpenChange,
  albumMenuOpen,
  onAlbumMenuOpenChange,
  onClear,
  rating,
  onRate,
  onToggleTag,
  onCreateTag,
  onToggleAlbum,
  onCreateAlbum,
  onExport,
  onCompare,
  onDelete,
  onRestore,
  onPurge,
  purgeEnabled = true,
  selectedTagKeys,
  selectedAlbumKeys,
}: {
  mode: FloatingBarMode;
  count: number;
  tags: TagDto[];
  albums: Album[];
  busy: boolean;
  tagMenuOpen: boolean;
  onTagMenuOpenChange: (open: boolean) => void;
  albumMenuOpen: boolean;
  onAlbumMenuOpenChange: (open: boolean) => void;
  onClear: () => void;
  rating: number | null;
  onRate: (rating: number) => void;
  onToggleTag: (tagId: number, add: boolean) => void | Promise<void>;
  onCreateTag: (tagName: string) => string | void | Promise<string | void>;
  onToggleAlbum: (albumId: number, add: boolean) => void | Promise<void>;
  onCreateAlbum: (name: string) => string | void | Promise<string | void>;
  onExport: () => void;
  onCompare: () => void;
  onDelete: () => void;
  onRestore: () => void;
  onPurge: () => void;
  purgeEnabled?: boolean;
  selectedTagKeys: string[];
  selectedAlbumKeys: string[];
}) {
  const { t } = useTranslation(["common", "library"]);
  const tagItems = useMemo(() => buildTagPickerItems(tags), [tags]);
  const albumItems = useMemo(() => buildAlbumPickerItems(albums), [albums]);

  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-6 z-30 flex justify-center px-4">
      <div
        className="surface-float pointer-events-auto flex max-w-full animate-slide-up items-center gap-1 px-2.5 py-1.5"
        role="toolbar"
        aria-label={t("common:aria.selectionActions")}
      >
        <span className="shrink-0 px-2 text-xs font-medium tabular-nums text-content-secondary">
          {t("common:count.selected", { count })}
        </span>

        <ToolbarDivider />

        <RatingRow compact value={rating} onSelect={onRate} />

        <ToolbarDivider />

        <div className="flex items-center">
          <SelectionPickerPopover
            label={t("library:selectionBar.tag")}
            shortcut={t("common:shortcut.tag")}
            items={tagItems}
            placeholder={t("library:selectionBar.newTag")}
            busy={busy}
            open={tagMenuOpen}
            onOpenChange={onTagMenuOpenChange}
            selectedKeys={selectedTagKeys}
            onToggle={(key, add) => onToggleTag(Number(key), add)}
            onCreate={onCreateTag}
          />
          <SelectionPickerPopover
            label={t("library:selectionBar.album")}
            shortcut={t("common:shortcut.album")}
            items={albumItems}
            placeholder={t("library:selectionBar.newAlbum")}
            busy={busy}
            open={albumMenuOpen}
            onOpenChange={onAlbumMenuOpenChange}
            selectedKeys={selectedAlbumKeys}
            onToggle={(key, add) => onToggleAlbum(Number(key), add)}
            onCreate={onCreateAlbum}
          />
          <button
            type="button"
            className="btn btn-ghost btn-interactive btn-xs h-7 min-h-0 px-2.5 text-xs font-normal"
            onClick={onExport}
          >
            {t("common:action.export")}
            <ShortcutHint>{t("common:shortcut.export")}</ShortcutHint>
          </button>
          {count >= 2 && (
            <button
              type="button"
              className="btn btn-ghost btn-interactive btn-xs h-7 min-h-0 px-2.5 text-xs font-normal"
              onClick={onCompare}
            >
              {t("common:action.compare")}
              <ShortcutHint>{t("common:shortcut.compare")}</ShortcutHint>
            </button>
          )}
          {mode === "trash" ? (
            <>
              <button
                type="button"
                className="btn btn-ghost btn-interactive btn-xs h-7 min-h-0 px-2.5 text-xs font-normal"
                disabled={busy}
                onClick={onRestore}
              >
                {t("common:action.restore")}
                <ShortcutHint>{t("common:shortcut.restore")}</ShortcutHint>
              </button>
              {purgeEnabled && (
                <button
                  type="button"
                  className="btn btn-ghost btn-interactive btn-xs h-7 min-h-0 px-2.5 text-xs font-normal text-danger-text hover:bg-danger-subtle hover:text-danger"
                  disabled={busy}
                  onClick={onPurge}
                >
                  {t("common:action.purge")}
                  <ShortcutHint>{t("common:shortcut.purge")}</ShortcutHint>
                </button>
              )}
            </>
          ) : (
            <button
              type="button"
              className="btn btn-ghost btn-interactive btn-xs h-7 min-h-0 px-2.5 text-xs font-normal text-danger-text hover:bg-danger-subtle hover:text-danger"
              onClick={onDelete}
            >
              {t("common:action.delete")}
              <ShortcutHint>{t("common:shortcut.delete")}</ShortcutHint>
            </button>
          )}
        </div>

        <ToolbarDivider />

        <button
          type="button"
          className="btn btn-ghost btn-interactive btn-xs btn-square h-7 min-h-0 w-7 shrink-0 text-content-faint hover:text-base-content"
          onClick={onClear}
          aria-label={t("common:aria.clearSelection")}
        >
          ×
        </button>
      </div>
    </div>
  );
}
