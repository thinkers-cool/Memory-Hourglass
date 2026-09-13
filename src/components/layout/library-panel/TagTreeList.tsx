import { useTranslation } from "react-i18next";
import { Plus, Trash2 } from "lucide-react";
import type { AssetFilter, TagDto } from "../../../types";
import type {
  FilterBarState,
  LibraryActions,
} from "../../../lib/libraryActions";
import { isTagSourceActive } from "../../../lib/libraryFilters";
import type { PanelOrderMode } from "../../../lib/panelOrder";
import { orderTagsForParent } from "../../../lib/panelOrder";
import { TagColorDot } from "../../shared/TagColorDot";
import { ItemCountSubtitle } from "./ItemCountSubtitle";
import { PanelListRow, PanelRowActionButton } from "./PanelListRow";
import { TagAddRow } from "./TagAddRow";
import { TagEditRow } from "./TagEditRow";
import { iconClass } from "./panelStyles";

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
      <PanelRowActionButton
        title={t("common:action.remove")}
        onClick={onDelete}
      >
        <Trash2 className={iconClass} />
      </PanelRowActionButton>
    </>
  );
}

export function TagTreeList({
  tags,
  parentId,
  depth,
  busy,
  filterBar,
  extraFilter,
  editingTagId,
  addingSubtagParentId,
  tagOrderMode,
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
  tagOrderMode: PanelOrderMode;
  actions: LibraryActions;
  onEdit: (tagId: number) => void;
  onCancelEdit: () => void;
  onStartAddSubtag: (tagId: number) => void;
  onCancelAddSubtag: () => void;
}) {
  const { t } = useTranslation("library");
  const items = orderTagsForParent(tags, parentId, tagOrderMode);

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
          badgeTitle={
            depth > 0 ? t("panel.badge.subtag") : t("panel.badge.tag")
          }
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
        tagOrderMode={tagOrderMode}
        actions={actions}
        onEdit={onEdit}
        onCancelEdit={onCancelEdit}
        onStartAddSubtag={onStartAddSubtag}
        onCancelAddSubtag={onCancelAddSubtag}
      />
    </div>
  ));
}
