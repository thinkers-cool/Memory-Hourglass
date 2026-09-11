import type { SetStateAction } from "react";
import { describe, expect, it, vi } from "vitest";
import type { FilterBarState } from "../../../lib/libraryActions";
import { emptyFilterBarState } from "../../../lib/libraryFilters";
import { makeLibraryActionsDeps } from "../../../test/createLibraryActionsDeps";
import { createFiltersActions } from "./filters";

describe("createFiltersActions", () => {
  it("routes filter actions through filter bar state", async () => {
    const setFilterBar = vi.fn((update: SetStateAction<FilterBarState>) => {
      if (typeof update === "function") {
        update(emptyFilterBarState("date", "desc"));
      }
    });
    const deps = makeLibraryActionsDeps({ setFilterBar });
    const actions = createFiltersActions(deps);

    actions.selectRoot(9);
    actions.clearFilters();
    actions.filterByTag(3);
    actions.viewTrash();
    actions.viewLibrary();
    await actions.selectAlbum(11);

    expect(deps.setSelectedCollectionId).toHaveBeenCalled();
    expect(deps.setFilterBar).toHaveBeenCalled();
    expect(deps.setExtraFilter).toHaveBeenCalled();
  });
});
