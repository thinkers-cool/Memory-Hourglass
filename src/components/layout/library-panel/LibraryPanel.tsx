import type {
  Album,
  AssetFilter,
  RootStats,
  SmartCollection,
  TagDto,
} from "../../../types";
import type {
  FilterBarState,
  LibraryActions,
} from "../../../lib/libraryActions";
import type { DeleteFilterStatus } from "../../../lib/libraryFilters";
import type { LeftTab } from "../NavRail";
import { CollectionsPanel } from "./CollectionsPanel";
import { SourcesPanel } from "./SourcesPanel";

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
