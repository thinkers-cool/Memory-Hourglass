import { useMemo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { RatingRow } from "../RatingRow";
import { TagColorDot } from "./TagColorDot";
import { albumEmoji } from "../../lib/libraryIndicators";
import { tagDepthFirst, tagPathLabel } from "../../lib/tagHierarchy";
import type { Album, TagDto } from "../../types";
import type { StampConfig } from "../../lib/stamp";
import {
  MENU_ITEM_BUTTON_CLASS,
  MENU_PICKER_LIST_CLASS,
} from "../../lib/formControlClass";
import { menuPickerItemClass } from "../../lib/optionRowClass";

function StampColumn({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <fieldset className="fieldset min-w-0 p-1">
      <legend className="fieldset-legend px-1 text-[10px] font-semibold uppercase tracking-wide text-content-faint">
        {label}
      </legend>
      {children}
    </fieldset>
  );
}

export function StampMegaDropdown({
  config,
  tags,
  albums,
  onRatingChange,
  onToggleTag,
  onToggleAlbum,
}: {
  config: StampConfig;
  tags: TagDto[];
  albums: Album[];
  onRatingChange: (rating: number | null) => void;
  onToggleTag: (tagId: number, add: boolean) => void;
  onToggleAlbum: (albumId: number, add: boolean) => void;
}) {
  const { t } = useTranslation("library");
  const orderedTags = useMemo(() => tagDepthFirst(tags), [tags]);

  return (
    <div
      className="surface-popover w-[min(40rem,calc(100vw-2rem))] p-0"
      role="dialog"
      aria-label={t("stamp.configuration")}
      onClick={(event) => event.stopPropagation()}
    >
      <div className="grid grid-cols-3 divide-x divide-divider-subtle">
        <StampColumn label={t("stamp.rating")}>
          <div className="px-1 pb-1">
            <RatingRow
              compact
              value={config.rating}
              onSelect={(rating) =>
                onRatingChange(config.rating === rating ? null : rating)
              }
            />
          </div>
        </StampColumn>

        <StampColumn label={t("stamp.tags")}>
          <ul className={`${MENU_PICKER_LIST_CLASS} max-h-48`}>
            {orderedTags.length === 0 ? (
              <li className="pointer-events-none px-2 py-1 text-xs opacity-45">
                {t("stamp.noTags")}
              </li>
            ) : (
              orderedTags.map((tag) => {
                const active = config.tag_ids.includes(tag.id);
                return (
                  <li key={tag.id}>
                    <button
                      type="button"
                      className={`${MENU_ITEM_BUTTON_CLASS} ${menuPickerItemClass(active)}`}
                      onClick={() => onToggleTag(tag.id, !active)}
                    >
                      <span className="flex w-4 shrink-0 items-center justify-center">
                        <TagColorDot color={tag.color} />
                      </span>
                      <span className="truncate">
                        {tagPathLabel(tag, tags)}
                      </span>
                    </button>
                  </li>
                );
              })
            )}
          </ul>
        </StampColumn>

        <StampColumn label={t("stamp.albums")}>
          <ul className={`${MENU_PICKER_LIST_CLASS} max-h-48`}>
            {albums.length === 0 ? (
              <li className="pointer-events-none px-2 py-1 text-xs opacity-45">
                {t("stamp.noAlbums")}
              </li>
            ) : (
              albums.map((album) => {
                const active = config.album_ids.includes(album.id);
                return (
                  <li key={album.id}>
                    <button
                      type="button"
                      className={`${MENU_ITEM_BUTTON_CLASS} ${menuPickerItemClass(active)}`}
                      onClick={() => onToggleAlbum(album.id, !active)}
                    >
                      <span className="flex w-4 shrink-0 items-center justify-center text-sm leading-none">
                        {albumEmoji(album.emoji)}
                      </span>
                      <span className="truncate">{album.name}</span>
                    </button>
                  </li>
                );
              })
            )}
          </ul>
        </StampColumn>
      </div>
    </div>
  );
}
