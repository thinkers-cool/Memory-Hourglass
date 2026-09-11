import { useTranslation } from "react-i18next";
import type { SmartCollection } from "../../../types";
import type { LibraryActions } from "../../../lib/libraryActions";
import { Bookmark, Trash2 } from "lucide-react";
import { PanelSection } from "../NavRail";
import { ItemCountSubtitle } from "./ItemCountSubtitle";
import { PanelListRow, PanelRowActionButton } from "./PanelListRow";
import { iconClass, panelShellClass } from "./panelStyles";

export function CollectionsPanel({
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
          <p className="text-xs opacity-40 px-2">
            {t("panel.empty.collections")}
          </p>
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
