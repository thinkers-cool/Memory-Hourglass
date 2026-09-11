import type { LibraryActions } from "../../lib/libraryActions";
import { createAlbumsActions } from "./actions/albums";
import { createCatalogActions } from "./actions/catalog";
import { createCollectionsActions } from "./actions/collections";
import { createCompareActions } from "./actions/compare";
import { createDeleteActions } from "./actions/delete";
import { createFiltersActions } from "./actions/filters";
import { createGalleryActions } from "./actions/gallery";
import { createRootsActions } from "./actions/roots";
import { createSelectionActions } from "./actions/selection";
import { createStampActions } from "./actions/stamp";
import { createTagsActions } from "./actions/tags";
import type { LibraryActionsDeps } from "./libraryActionsDeps";

export type { LibraryActionsDeps } from "./libraryActionsDeps";

export function createLibraryActions(deps: LibraryActionsDeps): LibraryActions {
  return {
    ...createRootsActions(deps),
    ...createFiltersActions(deps),
    ...createSelectionActions(deps),
    ...createTagsActions(deps),
    ...createAlbumsActions(deps),
    ...createDeleteActions(deps),
    ...createCollectionsActions(deps),
    ...createCompareActions(deps),
    ...createStampActions(deps),
    ...createGalleryActions(deps),
    ...createCatalogActions(deps),
  };
}
