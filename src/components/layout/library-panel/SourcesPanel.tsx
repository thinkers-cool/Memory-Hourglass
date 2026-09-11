import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Album, AssetFilter, RootStats, TagDto } from "../../../types";
import type {
  FilterBarState,
  LibraryActions,
} from "../../../lib/libraryActions";
import { albumEmoji } from "../../../lib/libraryIndicators";
import type { DeleteFilterStatus } from "../../../lib/libraryFilters";
import {
  isAlbumSourceActive,
  isRootSourceActive,
} from "../../../lib/libraryFilters";
import { FolderPlus, Network, Trash2 } from "lucide-react";
import { PanelSection } from "../NavRail";
import { AlbumAddRow } from "./AlbumAddRow";
import { AlbumEditRow } from "./AlbumEditRow";
import { ItemCountSubtitle } from "./ItemCountSubtitle";
import { PanelListRow, PanelRowActionButton } from "./PanelListRow";
import { SectionAddButton } from "./SectionAddButton";
import { SourceRow } from "./SourceRow";
import { TagAddRow } from "./TagAddRow";
import { TagTreeList } from "./TagTreeList";
import { tagRoots } from "../../../lib/tagHierarchy";
import { iconButtonClass, iconClass, panelShellClass } from "./panelStyles";

export function SourcesPanel({
  roots,
  albums,
  tags,
  filterBar,
  extraFilter,
  deleteStatus,
  deletedCount,
  busy,
  actions,
}: {
  roots: RootStats[];
  albums: Album[];
  tags: TagDto[];
  filterBar: FilterBarState;
  extraFilter: AssetFilter;
  deleteStatus: DeleteFilterStatus;
  deletedCount: number;
  busy: boolean;
  actions: LibraryActions;
}) {
  const [tagAdding, setTagAdding] = useState(false);
  const [addingSubtagParentId, setAddingSubtagParentId] = useState<
    number | null
  >(null);
  const [albumAdding, setAlbumAdding] = useState(false);
  const [editingAlbumId, setEditingAlbumId] = useState<number | null>(null);
  const [editingTagId, setEditingTagId] = useState<number | null>(null);
  const hasTags = tagRoots(tags).length > 0;
  const { t } = useTranslation(["library", "common"]);

  return (
    <div className={`${panelShellClass} overflow-auto p-3`} data-library-panel>
      <PanelSection
        title={t("panel.library")}
        action={
          <div className="flex shrink-0 items-center">
            <button
              type="button"
              className={iconButtonClass}
              title={t("panel.addFolder")}
              disabled={busy}
              onClick={actions.addLocalRoot}
            >
              <FolderPlus className={iconClass} />
            </button>
            <button
              type="button"
              className={iconButtonClass}
              title={t("panel.connectSmb")}
              disabled={busy}
              onClick={actions.openSmbConnect}
            >
              <Network className={iconClass} />
            </button>
          </div>
        }
      >
        {roots.length === 0 ? (
          <p className="text-xs opacity-40 px-2">{t("panel.empty.sources")}</p>
        ) : (
          roots.map((root) => (
            <SourceRow
              key={root.id}
              root={root}
              active={isRootSourceActive(root.id, filterBar, extraFilter)}
              onSelect={() => actions.selectRoot(root.id)}
              onSync={() => actions.syncRoot(root.id)}
              onRelink={() => actions.relinkRoot(root.id)}
              onRemove={() => actions.removeRoot(root.id)}
            />
          ))
        )}
      </PanelSection>

      <PanelSection
        title={t("panel.albums")}
        action={
          <SectionAddButton
            title={t("panel.newAlbum")}
            busy={busy}
            active={albumAdding}
            onClick={() => setAlbumAdding((prev) => !prev)}
          />
        }
      >
        {albumAdding && (
          <AlbumAddRow
            placeholder={t("panel.placeholder.newAlbum")}
            busy={busy}
            onCancel={() => setAlbumAdding(false)}
            onSubmit={(name, emoji) => {
              void actions.createAlbum(name, emoji);
              setAlbumAdding(false);
            }}
          />
        )}
        {albums.length === 0 && !albumAdding ? (
          <p className="text-xs opacity-40 px-2">{t("panel.empty.albums")}</p>
        ) : (
          albums.map((album) =>
            editingAlbumId === album.id ? (
              <AlbumEditRow
                key={album.id}
                album={album}
                busy={busy}
                onCancel={() => setEditingAlbumId(null)}
                onSave={(name, emoji) => {
                  void actions.updateAlbum(album.id, name, emoji);
                  setEditingAlbumId(null);
                }}
              />
            ) : (
              <PanelListRow
                key={album.id}
                active={isAlbumSourceActive(album.id, filterBar, extraFilter)}
                badge={
                  <span className="text-base leading-none">
                    {albumEmoji(album.emoji)}
                  </span>
                }
                badgeTitle={t("panel.badge.album")}
                title={album.name}
                subtitle={<ItemCountSubtitle count={album.asset_count} />}
                onClick={() => actions.selectAlbum(album.id)}
                onDoubleClick={() => setEditingAlbumId(album.id)}
                actions={
                  <PanelRowActionButton
                    title={t("common:action.remove")}
                    onClick={() => actions.deleteAlbum(album.id)}
                  >
                    <Trash2 className={iconClass} />
                  </PanelRowActionButton>
                }
              />
            ),
          )
        )}
      </PanelSection>

      <PanelSection
        title={t("panel.tags")}
        action={
          <SectionAddButton
            title={t("panel.newTag")}
            busy={busy}
            active={tagAdding}
            onClick={() => {
              setAddingSubtagParentId(null);
              setTagAdding((prev) => !prev);
            }}
          />
        }
      >
        {tagAdding && (
          <TagAddRow
            placeholder={t("panel.placeholder.newTag")}
            busy={busy}
            onCancel={() => setTagAdding(false)}
            onSubmit={(name, color) => {
              void actions.createTag(name, undefined, color);
              setTagAdding(false);
            }}
          />
        )}
        {!hasTags && !tagAdding ? (
          <p className="text-xs opacity-40 px-2">{t("panel.empty.tags")}</p>
        ) : (
          <TagTreeList
            tags={tags}
            parentId={null}
            depth={0}
            busy={busy}
            filterBar={filterBar}
            extraFilter={extraFilter}
            editingTagId={editingTagId}
            addingSubtagParentId={addingSubtagParentId}
            actions={actions}
            onEdit={setEditingTagId}
            onCancelEdit={() => setEditingTagId(null)}
            onStartAddSubtag={(tagId) => {
              setTagAdding(false);
              setAddingSubtagParentId(tagId);
            }}
            onCancelAddSubtag={() => setAddingSubtagParentId(null)}
          />
        )}
      </PanelSection>

      <PanelSection title={t("panel.trash")}>
        <PanelListRow
          active={deleteStatus === "deleted"}
          badge={<Trash2 className="h-4 w-4" aria-hidden />}
          badgeTitle={t("panel.badge.trash")}
          title={t("panel.deleted")}
          subtitle={<ItemCountSubtitle count={deletedCount} />}
          onClick={() => {
            if (deleteStatus === "deleted") actions.viewLibrary();
            else actions.viewTrash();
          }}
        />
      </PanelSection>
    </div>
  );
}
