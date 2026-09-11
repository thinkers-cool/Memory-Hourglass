import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Album, AssetFilter, RootStats, SmartCollection, TagDto } from "../../types";
import type { FilterBarState, LibraryActions } from "../../lib/libraryActions";
import { albumEmoji, DEFAULT_ALBUM_EMOJI, DEFAULT_TAG_COLOR } from "../../lib/libraryIndicators";
import type { DeleteFilterStatus } from "../../lib/libraryFilters";
import { tagChildren, tagRoots } from "../../lib/tagHierarchy";
import {
  isAlbumSourceActive,
  isRootSourceActive,
  isTagSourceActive,
} from "../../lib/libraryFilters";
import { ColorPickerPopover } from "../shared/ColorPickerPopover";
import { EmojiPickerPopover } from "../shared/EmojiPickerPopover";
import { TagColorDot } from "../shared/TagColorDot";
import {
  Folder,
  FolderInput,
  FolderPlus,
  Bookmark,
  Network,
  Plus,
  Check,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { PanelSection } from "./NavRail";
import type { LeftTab } from "./NavRail";
import { ghostBtnClass } from "../../lib/buttonClass";
import {
  INPUT_CONTROL_CLASS,
} from "../../lib/formControlClass";
import {
  listRowBadgeClass,
  listRowClass,
  listRowTitleClass,
} from "../../lib/interactionClass";

const iconClass = "h-3.5 w-3.5";
const iconButtonClass = ghostBtnClass("btn-xs btn-square h-8 min-h-0 w-8 shrink-0");
const panelRowActionButtonClass = ghostBtnClass(
  "btn-xs btn-square h-7 min-h-0 w-7 shrink-0 text-content-muted hover:text-base-content",
);
function panelRowButtonClass(active?: boolean, dimmed?: boolean) {
  return `${ghostBtnClass("h-10 min-h-0 w-full min-w-0 items-center justify-start gap-2 rounded-lg p-1 text-left font-normal")} ${listRowClass(active)} ${
    dimmed ? "opacity-80" : ""
  }`;
}
function rootDisplayName(path: string): string {
  const segments = path.split("/");
  return segments[segments.length - 1] || path;
}
function ItemCountSubtitle({ count }: { count: number }) {
  const { t } = useTranslation("library");
  return (
    <span className="tabular-nums">
      {t("panel.items", { count })}
    </span>
  );
}
const inputClass = `${INPUT_CONTROL_CLASS} min-w-0 flex-1 px-2 text-xs`;
const panelShellClass =
  "flex h-full flex-col border-r surface-panel";

function PanelRowBadge({
  active,
  title,
  children,
}: {
  active?: boolean;
  title?: string;
  children: React.ReactNode;
}) {
  return (
    <span
      title={title}
      className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-lg ${listRowBadgeClass(active)}`}
    >
      {children}
    </span>
  );
}

function PanelRowActionButton({
  title,
  onClick,
  children,
}: {
  title: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      className={panelRowActionButtonClass}
      title={title}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
    >
      {children}
    </button>
  );
}

function PanelRowActions({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="surface-chip pointer-events-none absolute right-1 top-1/2 flex -translate-y-1/2 items-center gap-0.5 px-0.5 opacity-0 transition-opacity group-hover:pointer-events-auto group-hover:opacity-100"
    >
      {children}
    </div>
  );
}

function PanelListRow({
  active,
  dimmed,
  indented,
  badge,
  badgeTitle,
  title,
  titleTooltip,
  subtitle,
  onClick,
  onDoubleClick,
  actions,
}: {
  active?: boolean;
  dimmed?: boolean;
  indented?: boolean;
  badge: React.ReactNode;
  badgeTitle?: string;
  title: string;
  titleTooltip?: string;
  subtitle: React.ReactNode;
  onClick: () => void;
  onDoubleClick?: () => void;
  actions?: React.ReactNode;
}) {
  return (
    <div className={`group relative mb-1 last:mb-0 ${indented ? "ml-3" : ""}`}>
      <button
        type="button"
        className={panelRowButtonClass(active, dimmed)}
        onClick={onClick}
        onDoubleClick={onDoubleClick}
      >
        <PanelRowBadge active={active} title={badgeTitle}>
          {badge}
        </PanelRowBadge>
        <span className="flex min-w-0 flex-1 flex-col gap-0.5 overflow-hidden pr-1">
          <span
            className={`truncate text-xs leading-tight ${listRowTitleClass(active)}`}
            title={titleTooltip ?? title}
          >
            {title}
          </span>
          <span className="flex items-center gap-1.5 text-[10px] leading-none text-content-tertiary">
            {subtitle}
          </span>
        </span>
      </button>
      {actions ? <PanelRowActions>{actions}</PanelRowActions> : null}
    </div>
  );
}

function SectionAddButton({
  title,
  busy,
  active,
  onClick,
}: {
  title: string;
  busy: boolean;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`${iconButtonClass} ${active ? "bg-interactive-hover-strong" : ""}`}
      title={title}
      disabled={busy}
      onClick={onClick}
    >
      <Plus className={iconClass} />
    </button>
  );
}

function AlbumAddRow({
  placeholder,
  busy,
  onSubmit,
  onCancel,
}: {
  placeholder: string;
  busy: boolean;
  onSubmit: (name: string, emoji: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("common");
  const [value, setValue] = useState("");
  const [emoji, setEmoji] = useState(DEFAULT_ALBUM_EMOJI);

  const commit = () => {
    const trimmed = value.trim();
    if (!trimmed) return;
    onSubmit(trimmed, emoji);
    setValue("");
    setEmoji(DEFAULT_ALBUM_EMOJI);
  };

  return (
    <div className="mb-1 flex h-8 items-center gap-1.5 px-1">
      <EmojiPickerPopover value={emoji} disabled={busy} onChange={setEmoji} />
      <input
        type="text"
        className={inputClass}
        placeholder={placeholder}
        value={value}
        autoFocus
        disabled={busy}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") onCancel();
        }}
      />
      <button
        type="button"
        className="btn btn-primary btn-sm btn-square h-8 min-h-0 w-8"
        title={t("action.save")}
        disabled={busy || !value.trim()}
        onClick={commit}
      >
        <Check className={iconClass} />
      </button>
    </div>
  );
}

function TagAddRow({
  placeholder,
  busy,
  indented = false,
  onSubmit,
  onCancel,
}: {
  placeholder: string;
  busy: boolean;
  indented?: boolean;
  onSubmit: (name: string, color: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("common");
  const [value, setValue] = useState("");
  const [color, setColor] = useState(DEFAULT_TAG_COLOR);

  const commit = () => {
    const trimmed = value.trim();
    if (!trimmed) return;
    onSubmit(trimmed, color);
    setValue("");
    setColor(DEFAULT_TAG_COLOR);
  };

  return (
    <div className={`mb-1 flex h-8 items-center gap-1.5 px-1 ${indented ? "ml-3" : ""}`}>
      <ColorPickerPopover value={color} disabled={busy} onChange={setColor} />
      <input
        type="text"
        className={inputClass}
        placeholder={placeholder}
        value={value}
        autoFocus
        disabled={busy}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") onCancel();
        }}
      />
      <button
        type="button"
        className="btn btn-primary btn-sm btn-square h-8 min-h-0 w-8"
        title={t("action.save")}
        disabled={busy || !value.trim()}
        onClick={commit}
      >
        <Check className={iconClass} />
      </button>
    </div>
  );
}

function SourceRow({
  root,
  active,
  onSelect,
  onSync,
  onRelink,
  onRemove,
}: {
  root: RootStats;
  active?: boolean;
  onSelect: () => void;
  onSync: () => void;
  onRelink: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation(["library", "common"]);
  const name = rootDisplayName(root.path);
  const offline = root.status === "offline";
  const isSmb = root.kind === "smb";
  const Icon = isSmb ? Network : Folder;

  return (
    <PanelListRow
      active={active}
      dimmed={offline}
      badge={<Icon className="h-4 w-4" aria-hidden />}
      badgeTitle={isSmb ? t("panel.badge.networkSource") : t("panel.badge.localFolder")}
      title={name}
      titleTooltip={root.path}
      subtitle={
        <>
          {offline ? (
            <span className="font-medium text-warning">{t("panel.offline")}</span>
          ) : null}
          <ItemCountSubtitle count={root.asset_count} />
        </>
      }
      onClick={onSelect}
      actions={
        <>
          <PanelRowActionButton title={t("panel.sync")} onClick={onSync}>
            <RefreshCw className={iconClass} />
          </PanelRowActionButton>
          <PanelRowActionButton title={t("panel.relink")} onClick={onRelink}>
            <FolderInput className={iconClass} />
          </PanelRowActionButton>
          <PanelRowActionButton title={t("common:action.remove")} onClick={onRemove}>
            <Trash2 className={iconClass} />
          </PanelRowActionButton>
        </>
      }
    />
  );
}

function AlbumEditRow({
  album,
  busy,
  onSave,
  onCancel,
}: {
  album: Album;
  busy: boolean;
  onSave: (name: string, emoji: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("common");
  const [name, setName] = useState(album.name);
  const [emoji, setEmoji] = useState(album.emoji ?? DEFAULT_ALBUM_EMOJI);

  const commit = () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    onSave(trimmed, emoji);
  };

  return (
    <div className="mb-1 flex h-8 items-center gap-1.5 px-1">
      <EmojiPickerPopover value={emoji} disabled={busy} onChange={setEmoji} />
      <input
        type="text"
        className={inputClass}
        value={name}
        autoFocus
        disabled={busy}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") onCancel();
        }}
      />
      <button
        type="button"
        className="btn btn-primary btn-sm btn-square h-8 min-h-0 w-8"
        title={t("action.save")}
        disabled={busy || !name.trim()}
        onClick={commit}
      >
        <Check className={iconClass} />
      </button>
    </div>
  );
}

function TagEditRow({
  tag,
  busy,
  onSave,
  onCancel,
}: {
  tag: TagDto;
  busy: boolean;
  onSave: (name: string, color: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("common");
  const [name, setName] = useState(tag.name);
  const [color, setColor] = useState(tag.color ?? DEFAULT_TAG_COLOR);

  const commit = () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    onSave(trimmed, color);
  };

  return (
    <div className="mb-1 flex h-8 items-center gap-1.5 px-1">
      <ColorPickerPopover value={color} disabled={busy} onChange={setColor} />
      <input
        type="text"
        className={inputClass}
        value={name}
        autoFocus
        disabled={busy}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") onCancel();
        }}
      />
      <button
        type="button"
        className="btn btn-primary btn-sm btn-square h-8 min-h-0 w-8"
        title={t("action.save")}
        disabled={busy || !name.trim()}
        onClick={commit}
      >
        <Check className={iconClass} />
      </button>
    </div>
  );
}

function CollectionsPanel({
  collections,
  selectedCollectionId,
  actions,
}: {
  collections: SmartCollection[];
  selectedCollectionId: number | null;
  actions: LibraryActions;
}) {
  const { t } = useTranslation(["library", "common"]);

  return (
    <div className={`${panelShellClass} overflow-auto p-3`}>
      <PanelSection title={t("panel.collections")}>
        {collections.length === 0 ? (
          <p className="text-xs opacity-40 px-2">{t("panel.empty.collections")}</p>
        ) : (
          collections.map((collection) => (
            <PanelListRow
              key={collection.id}
              active={selectedCollectionId === collection.id}
              badge={<Bookmark className="h-4 w-4" aria-hidden />}
              badgeTitle={t("panel.badge.smartCollection")}
              title={collection.name}
              subtitle={<ItemCountSubtitle count={collection.asset_count} />}
              onClick={() => actions.selectCollection(collection)}
              actions={
                <PanelRowActionButton
                  title={t("common:action.remove")}
                  onClick={() => actions.deleteCollection(collection.id)}
                >
                  <Trash2 className={iconClass} />
                </PanelRowActionButton>
              }
            />
          ))
        )}
      </PanelSection>
    </div>
  );
}

function TagRowActions({
  onAddSubtag,
  onDelete,
}: {
  onAddSubtag: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation(["library", "common"]);

  return (
    <>
      <PanelRowActionButton title={t("panel.addSubtag")} onClick={onAddSubtag}>
        <Plus className={iconClass} />
      </PanelRowActionButton>
      <PanelRowActionButton title={t("common:action.remove")} onClick={onDelete}>
        <Trash2 className={iconClass} />
      </PanelRowActionButton>
    </>
  );
}

function TagTreeList({
  tags,
  parentId,
  depth,
  busy,
  filterBar,
  extraFilter,
  editingTagId,
  addingSubtagParentId,
  actions,
  onEdit,
  onCancelEdit,
  onStartAddSubtag,
  onCancelAddSubtag,
}: {
  tags: TagDto[];
  parentId: number | null;
  depth: number;
  busy: boolean;
  filterBar: FilterBarState;
  extraFilter: AssetFilter;
  editingTagId: number | null;
  addingSubtagParentId: number | null;
  actions: LibraryActions;
  onEdit: (tagId: number) => void;
  onCancelEdit: () => void;
  onStartAddSubtag: (tagId: number) => void;
  onCancelAddSubtag: () => void;
}) {
  const { t } = useTranslation("library");
  const items = parentId === null ? tagRoots(tags) : tagChildren(tags, parentId);

  return items.map((tag) => (
    <div key={tag.id}>
      {editingTagId === tag.id ? (
        <TagEditRow
          tag={tag}
          busy={busy}
          onCancel={onCancelEdit}
          onSave={(name, color) => {
            void actions.updateTag(tag.id, name, color);
            onCancelEdit();
          }}
        />
      ) : (
        <PanelListRow
          indented={depth > 0}
          active={isTagSourceActive(tag.id, filterBar, extraFilter)}
          badge={<TagColorDot color={tag.color} className="h-3 w-3" />}
          badgeTitle={depth > 0 ? t("panel.badge.subtag") : t("panel.badge.tag")}
          title={tag.name}
          subtitle={<ItemCountSubtitle count={tag.asset_count} />}
          onClick={() => actions.filterByTag(tag.id)}
          onDoubleClick={() => onEdit(tag.id)}
          actions={
            <TagRowActions
              onAddSubtag={() => onStartAddSubtag(tag.id)}
              onDelete={() => actions.deleteTag(tag.id)}
            />
          }
        />
      )}
      {addingSubtagParentId === tag.id ? (
        <TagAddRow
          indented
          placeholder={t("panel.placeholder.newSubtag")}
          busy={busy}
          onCancel={onCancelAddSubtag}
          onSubmit={(name, color) => {
            void actions.createTag(name, tag.id, color);
            onCancelAddSubtag();
          }}
        />
      ) : null}
      <TagTreeList
        tags={tags}
        parentId={tag.id}
        depth={depth + 1}
        busy={busy}
        filterBar={filterBar}
        extraFilter={extraFilter}
        editingTagId={editingTagId}
        addingSubtagParentId={addingSubtagParentId}
        actions={actions}
        onEdit={onEdit}
        onCancelEdit={onCancelEdit}
        onStartAddSubtag={onStartAddSubtag}
        onCancelAddSubtag={onCancelAddSubtag}
      />
    </div>
  ));
}

function SourcesPanel({
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
  const [addingSubtagParentId, setAddingSubtagParentId] = useState<number | null>(null);
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


export function LibraryPanel({
  tab,
  roots,
  albums,
  collections,
  tags,
  deletedCount,
  filterBar,
  extraFilter,
  selectedCollectionId,
  deleteStatus,
  busy,
  actions,
}: {
  tab: LeftTab;
  roots: RootStats[];
  albums: Album[];
  collections: SmartCollection[];
  tags: TagDto[];
  deletedCount: number;
  filterBar: FilterBarState;
  extraFilter: AssetFilter;
  selectedCollectionId: number | null;
  deleteStatus: DeleteFilterStatus;
  busy: boolean;
  actions: LibraryActions;
}) {
  if (tab === "library") {
    return (
      <SourcesPanel
        roots={roots}
        albums={albums}
        tags={tags}
        filterBar={filterBar}
        extraFilter={extraFilter}
        deleteStatus={deleteStatus}
        deletedCount={deletedCount}
        busy={busy}
        actions={actions}
      />
    );
  }

  if (tab === "collections") {
    return (
      <CollectionsPanel
        collections={collections}
        selectedCollectionId={selectedCollectionId}
        actions={actions}
      />
    );
  }

  return null;
}
