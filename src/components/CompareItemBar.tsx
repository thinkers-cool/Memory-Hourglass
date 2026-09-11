import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { RatingRow } from "./RatingRow";
import { SelectionPickerPopover } from "./shared/SelectionPickerPopover";
import type { Album, TagDto } from "../types";
import {
  buildAlbumPickerItems,
  buildTagPickerItems,
  ToolbarDivider,
} from "../lib/pickerItems";

export function CompareItemBar({
  rating,
  tags,
  albums,
  busy,
  tagMenuOpen,
  onTagMenuOpenChange,
  albumMenuOpen,
  onAlbumMenuOpenChange,
  onRate,
  onToggleTag,
  onCreateTag,
  onToggleAlbum,
  onCreateAlbum,
  onExport,
  onDelete,
  selectedTagKeys,
  selectedAlbumKeys,
}: {
  rating: number | null;
  tags: TagDto[];
  albums: Album[];
  busy: boolean;
  tagMenuOpen: boolean;
  onTagMenuOpenChange: (open: boolean) => void;
  albumMenuOpen: boolean;
  onAlbumMenuOpenChange: (open: boolean) => void;
  onRate: (rating: number) => void;
  onToggleTag: (tagId: number, add: boolean) => void | Promise<void>;
  onCreateTag: (tagName: string) => string | void | Promise<string | void>;
  onToggleAlbum: (albumId: number, add: boolean) => void | Promise<void>;
  onCreateAlbum: (name: string) => string | void | Promise<string | void>;
  onExport: () => void;
  onDelete: () => void;
  selectedTagKeys: string[];
  selectedAlbumKeys: string[];
}) {
  const { t } = useTranslation(["common", "library"]);
  const tagItems = useMemo(() => buildTagPickerItems(tags), [tags]);
  const albumItems = useMemo(() => buildAlbumPickerItems(albums), [albums]);

  return (
    <div
      className="surface-float pointer-events-auto flex max-w-full items-center gap-1 px-2 py-1"
      role="toolbar"
      aria-label={t("common:aria.itemActions")}
    >
      <RatingRow compact value={rating} onSelect={onRate} />
      <ToolbarDivider />
      <SelectionPickerPopover
        label={t("library:selectionBar.tag")}
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
        items={albumItems}
        placeholder={t("library:selectionBar.newAlbum")}
        busy={busy}
        open={albumMenuOpen}
        onOpenChange={onAlbumMenuOpenChange}
        selectedKeys={selectedAlbumKeys}
        onToggle={(key, add) => onToggleAlbum(Number(key), add)}
        onCreate={onCreateAlbum}
      />
      <ToolbarDivider />
      <button
        type="button"
        className="btn btn-ghost btn-interactive btn-xs h-7 min-h-0 px-2.5 text-xs font-normal"
        onClick={onExport}
      >
        {t("common:action.export")}
      </button>
      <button
        type="button"
        className="btn btn-ghost btn-interactive btn-xs h-7 min-h-0 px-2.5 text-xs font-normal text-error"
        onClick={onDelete}
      >
        {t("common:action.delete")}
      </button>
    </div>
  );
}
