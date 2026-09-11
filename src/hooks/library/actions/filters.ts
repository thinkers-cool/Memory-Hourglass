import { emptyFilterBarState } from "../../../lib/libraryFilters";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createFiltersActions(deps: LibraryActionsDeps) {
  const { setSelectedCollectionId, setFilterBar, setExtraFilter } = deps;

  return {
    selectRoot: (id: number) => {
      setSelectedCollectionId(null);
      setFilterBar((prev) => emptyFilterBarState(prev.sort, prev.sortDir));
      setExtraFilter({ root_id: id });
    },
    clearFilters: () => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => emptyFilterBarState(prev.sort, prev.sortDir));
    },
    filterByTag: (tagId: number) => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => ({
        ...emptyFilterBarState(prev.sort, prev.sortDir),
        tagIds: [tagId],
      }));
    },
    viewTrash: () => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => ({
        ...emptyFilterBarState(prev.sort, prev.sortDir),
        deleteStatus: "deleted",
      }));
    },
    viewLibrary: () => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => emptyFilterBarState(prev.sort, prev.sortDir));
    },
    selectAlbum: async (albumId: number) => {
      setSelectedCollectionId(null);
      setExtraFilter({});
      setFilterBar((prev) => ({
        ...emptyFilterBarState(prev.sort, prev.sortDir),
        albumIds: [albumId],
      }));
    },
  };
}
