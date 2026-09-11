import { beforeEach, describe, expect, it, vi } from "vitest";
import { createLibraryActions } from "./createLibraryActions";
import { EMPTY_STAMP_CONFIG } from "../../lib/stamp";
import {
  makeLibraryActionsDeps,
  sampleActionDetail,
  sampleActionItem,
  sampleActionItem2,
} from "../../test/createLibraryActionsDeps";

vi.mock("../../api/client", () => ({
  addRoot: vi.fn(),
  addSmbSource: vi.fn(),
  connectSmbShare: vi.fn(),
  startScan: vi.fn(),
  removeRoot: vi.fn(),
  getAsset: vi.fn(),
  updateAssetMeta: vi.fn(),
  batchUpdateAssetMeta: vi.fn(),
  softDeleteAssets: vi.fn(),
  purgeDelete: vi.fn(),
  restoreAssets: vi.fn(),
  createAlbum: vi.fn(),
  setAlbumItems: vi.fn(),
  deleteAlbum: vi.fn(),
  deleteSmartCollection: vi.fn(),
  saveSmartCollection: vi.fn(),
  previewRelink: vi.fn(),
  relinkRoot: vi.fn(),
  createTag: vi.fn(),
  updateTag: vi.fn(),
  deleteTag: vi.fn(),
  updateAlbum: vi.fn(),
  rebuildCatalog: vi.fn(),
  cancelScan: vi.fn(),
  queryAssets: vi.fn(),
  batchAppendTags: vi.fn(),
  batchRemoveTags: vi.fn(),
  addAlbumItems: vi.fn(),
  removeAlbumItems: vi.fn(),
}));

vi.mock("../../lib/pickFolder", () => ({
  pickFolder: vi.fn(),
}));

vi.mock("../../lib/libraryMutations", () => ({
  runTagToggle: vi.fn(),
  runTagCreate: vi.fn(),
  runAlbumToggle: vi.fn(),
  runAlbumCreate: vi.fn(),
}));

vi.mock("../../lib/stampMutations", () => ({
  runStampToggle: vi.fn(),
}));

import * as api from "../../api/client";
import { pickFolder } from "../../lib/pickFolder";
import {
  runAlbumCreate,
  runAlbumToggle,
  runTagCreate,
  runTagToggle,
} from "../../lib/libraryMutations";
import { runStampToggle } from "../../lib/stampMutations";
import * as gridNavigation from "../../lib/gridNavigation";
import * as gridNavigation from "../../lib/gridNavigation";

describe("createLibraryActions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.getAsset).mockResolvedValue(sampleActionDetail);
    vi.mocked(api.updateAssetMeta).mockResolvedValue(sampleActionDetail);
    vi.mocked(api.startScan).mockResolvedValue(undefined);
    vi.mocked(api.restoreAssets).mockResolvedValue(1);
    vi.mocked(api.queryAssets).mockResolvedValue({
      items: [sampleActionItem, sampleActionItem2],
      total: 2,
    });
    vi.mocked(runTagToggle).mockResolvedValue(1);
    vi.mocked(runTagCreate).mockResolvedValue(9);
    vi.mocked(runAlbumToggle).mockResolvedValue(1);
    vi.mocked(runAlbumCreate).mockResolvedValue(5);
    vi.mocked(runStampToggle).mockResolvedValue({
      applied: 1,
      unstamped: 0,
      stampedIds: [1],
      unstampedIds: [],
    });
  });

  it("wires roots and smb flows", async () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    actions.addLocalRoot();
    expect(deps.addRootAndScan).toHaveBeenCalled();

    actions.openSmbConnect();
    expect(deps.setSmbDialogOpen).toHaveBeenCalledWith(true);

    vi.mocked(api.addSmbSource).mockResolvedValue({
      id: 3,
      path: "/smb",
      kind: "smb",
    } as never);
    await actions.addMountedSmbPath("/smb");
    expect(deps.refreshMeta).toHaveBeenCalled();

    vi.mocked(api.connectSmbShare).mockResolvedValue({
      id: 4,
      path: "/smb2",
      kind: "smb",
    } as never);
    await actions.connectSmbShare({
      host: "h",
      share: "s",
      username: "u",
      password: "p",
      pollSecs: 60,
      folderPath: "/x",
    });
    expect(deps.setSmbDialogOpen).toHaveBeenCalledWith(false);

    vi.mocked(api.addSmbSource).mockRejectedValue(new Error("smb fail"));
    await actions.addMountedSmbPath("/bad");
    expect(deps.setNotification).toHaveBeenCalled();

    await actions.removeRoot(1);
    expect(api.removeRoot).toHaveBeenCalledWith(1);
    await actions.syncRoot(2);
    expect(api.startScan).toHaveBeenCalledWith(2);
  });

  it("updates filters and selection", async () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    actions.selectRoot(5);
    actions.clearFilters();
    actions.filterByTag(7);
    actions.viewTrash();
    actions.viewLibrary();
    await actions.selectAlbum(3);
    expect(deps.setExtraFilter).toHaveBeenCalled();
    expect(deps.setFilterBar).toHaveBeenCalled();
    expect(deps.setSelectedCollectionId).toHaveBeenCalled();

    await actions.selectAsset(sampleActionItem2, true, false);
    expect(deps.setSelectedIds).toHaveBeenCalled();
    expect(deps.setDetail).toHaveBeenCalled();

    const emptyDeps = makeLibraryActionsDeps({ items: [], selectedList: [] });
    await createLibraryActions(emptyDeps).selectAsset(
      sampleActionItem,
      false,
      false,
    );
    expect(emptyDeps.setSelectedIds).not.toHaveBeenCalled();

    const clearDeps = makeLibraryActionsDeps({
      setSelectedIds: vi.fn((updater) => {
        if (typeof updater === "function") {
          updater(new Set());
        }
      }),
    });
    await createLibraryActions(clearDeps).selectAsset(
      sampleActionItem,
      false,
      false,
    );
  });

  it("handles full view and linked asset navigation", async () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    await actions.openFullViewFor(sampleActionItem);
    expect(deps.setFullView).toHaveBeenCalledWith(true);

    await actions.openFullView();
    actions.closeFullView();
    await actions.openLinked(2);
    expect(deps.setInspectorVisible).toHaveBeenCalled();
  });

  it("rates assets individually and in batch", async () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    await actions.rate(4);
    await actions.batchRate(3);
    expect(api.updateAssetMeta).toHaveBeenCalled();
    expect(api.batchUpdateAssetMeta).toHaveBeenCalled();
  });

  it("opens tag and album menus when selection exists", () => {
    const deps = makeLibraryActionsDeps({ selectedList: [] });
    const actions = createLibraryActions(deps);
    actions.openTagMenu();
    actions.openAlbumMenu();
    expect(deps.setTagMenuOpen).not.toHaveBeenCalled();

    const withSelection = makeLibraryActionsDeps();
    createLibraryActions(withSelection).openTagMenu();
    createLibraryActions(withSelection).openAlbumMenu();
    expect(withSelection.setTagMenuOpen).toHaveBeenCalledWith(true);
    expect(withSelection.setAlbumMenuOpen).toHaveBeenCalledWith(true);
  });

  it("mutates tags and albums on selection", async () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    await actions.toggleTagOnSelection(3, true);
    await actions.createTagOnSelection(" trip ");
    expect(runTagToggle).toHaveBeenCalled();
    expect(runTagCreate).toHaveBeenCalled();
    await actions.toggleAlbumOnSelection(4, true);
    await actions.createAlbumOnSelection(" set ");
    expect(runAlbumToggle).toHaveBeenCalled();
    expect(runAlbumCreate).toHaveBeenCalled();
  });

  it("soft deletes, purges, and restores", async () => {
    const deps = makeLibraryActionsDeps({ purgeTargetIds: [1, 2] });
    const actions = createLibraryActions(deps);
    await actions.batchRemove();
    await actions.softDelete();
    actions.purge();
    actions.batchPurge();
    await actions.submitPurge();
    await actions.restoreSelected();
    expect(api.softDeleteAssets).toHaveBeenCalled();
    expect(api.purgeDelete).toHaveBeenCalledWith([1, 2], "DELETE");
    expect(api.restoreAssets).toHaveBeenCalled();
  });

  it("blocks purge actions in read-only workspaces", async () => {
    const deps = makeLibraryActionsDeps({
      purgeTargetIds: [1, 2],
      readOnly: true,
    });
    const actions = createLibraryActions(deps);
    actions.purge();
    actions.batchPurge();
    await actions.submitPurge();
    expect(api.purgeDelete).not.toHaveBeenCalled();
    expect(deps.setPurgeDialogOpen).not.toHaveBeenCalled();
    expect(deps.setPurgeTargetIds).not.toHaveBeenCalled();
  });

  it("creates albums and collections", async () => {
    vi.mocked(api.createAlbum).mockResolvedValue({
      id: 8,
      name: "a",
      emoji: null,
      is_smart: false,
    });
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    await actions.createAlbum(" album ", "📷");
    actions.selectAlbum(8);
    actions.selectCollection({
      id: 1,
      name: "sc",
      filter: { tag_ids: [1] },
    } as never);
    actions.saveCollection();
    await actions.submitSaveCollection(" saved ");
    expect(api.createAlbum).toHaveBeenCalled();
    expect(api.saveSmartCollection).toHaveBeenCalled();
  });

  it("confirms destructive album, tag, collection, catalog actions", async () => {
    const deps = makeLibraryActionsDeps({
      stampConfig: { ...EMPTY_STAMP_CONFIG },
    });
    const actions = createLibraryActions(deps);
    actions.deleteAlbum(1);
    actions.deleteTag(2);
    actions.deleteCollection(3);
    actions.rebuildCatalog();
    expect(deps.requestConfirm).toHaveBeenCalledTimes(4);

    const [, , onConfirm] = deps.requestConfirm.mock.calls[0];
    await onConfirm();
    expect(api.deleteAlbum).toHaveBeenCalledWith(1);
  });

  it("relinks roots and handles picker errors", async () => {
    vi.mocked(pickFolder).mockResolvedValue("/new");
    vi.mocked(api.previewRelink).mockResolvedValue({
      matched: 1,
      total_sampled: 1,
      new_path: "/new",
    });
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    await actions.relinkRoot(1);
    expect(deps.requestConfirm).toHaveBeenCalled();

    vi.mocked(pickFolder).mockRejectedValue(new Error("picker"));
    await actions.relinkRoot(1);
    vi.mocked(pickFolder).mockResolvedValue(null);
    await actions.relinkRoot(1);
  });

  it("opens gallery and clears selection", () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    document.startViewTransition = vi.fn((cb: () => void) => {
      cb();
      return {} as ViewTransition;
    });
    actions.openGallery();
    actions.closeInspector();
    actions.closeDetail();
    actions.clearSelection();
    expect(deps.setGalleryIndex).toHaveBeenCalled();

    const empty = makeLibraryActionsDeps({ items: [] });
    createLibraryActions(empty).openGallery();
  });

  it("manages tags and albums metadata", async () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    await actions.createTag("tag", 1, "#fff");
    await actions.updateTag(1, "tag2");
    await actions.updateAlbum(2, "album");
    expect(api.createTag).toHaveBeenCalled();
    expect(api.updateTag).toHaveBeenCalled();
    expect(api.updateAlbum).toHaveBeenCalled();
  });

  it("navigates grid and toggles fullscreen", async () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    actions.navigateGrid(1, 0);
    actions.navigateRelative(1);
    actions.adjustGridSize(1);
    actions.toggleFullscreen();
    expect(deps.setSelectedId).toHaveBeenCalled();
    expect(deps.setGridColumnCount).toHaveBeenCalled();
  });

  it("opens and closes compare mode", async () => {
    const single = makeLibraryActionsDeps({ selectedList: [1] });
    createLibraryActions(single).openCompare();
    expect(single.setNotification).toHaveBeenCalled();

    const deps = makeLibraryActionsDeps({ selectedList: [1, 2] });
    const actions = createLibraryActions(deps);
    await actions.openCompare();
    actions.closeCompare();
    expect(deps.setCompareOpen).toHaveBeenCalledWith(true);
    expect(deps.setCompareOpen).toHaveBeenCalledWith(false);
  });

  it("mutates compare assets and deletes from compare", async () => {
    const deps = makeLibraryActionsDeps({
      compareOpen: true,
      compareItems: [sampleActionItem, sampleActionItem2],
      selectedList: [1, 2],
    });
    const actions = createLibraryActions(deps);
    await actions.rateAsset(1, 5);
    await actions.toggleTagOnAsset(1, 2, true);
    await actions.createTagOnAsset(1, "x");
    await actions.toggleAlbumOnAsset(1, 3, true);
    await actions.createAlbumOnAsset(1, "album");
    await actions.softDeleteDuplicate(2);
    await actions.deleteAsset(2);
    expect(api.updateAssetMeta).toHaveBeenCalled();
    expect(api.softDeleteAssets).toHaveBeenCalled();
  });

  it("stamps targets and handles guard paths", async () => {
    const disarmed = makeLibraryActionsDeps({ stampArmed: false });
    await createLibraryActions(disarmed).toggleStampOnTargets();
    expect(runStampToggle).not.toHaveBeenCalled();

    const invalid = makeLibraryActionsDeps({
      stampConfig: { ...EMPTY_STAMP_CONFIG },
    });
    await createLibraryActions(invalid).toggleStampOnTargets();

    const noTargets = makeLibraryActionsDeps({
      selectedId: null,
      selectedList: [],
      stampArmed: true,
      stampConfig: { ...EMPTY_STAMP_CONFIG, rating: 1 },
    });
    await createLibraryActions(noTargets).toggleStampOnTargets();

    const deps = makeLibraryActionsDeps({
      compareOpen: true,
      compareItems: [sampleActionItem],
      compareDetails: { 1: { tag_ids: [1], album_ids: [2] } },
    });
    await createLibraryActions(deps).toggleStampOnTargets();
    expect(runStampToggle).toHaveBeenCalled();

    vi.mocked(runStampToggle).mockResolvedValueOnce({
      applied: 0,
      unstamped: 1,
      stampedIds: [],
      unstampedIds: [1],
    });
    await createLibraryActions(deps).toggleStampOnTargets();
  });

  it("cancels scan and handles scan start errors", async () => {
    const deps = makeLibraryActionsDeps();
    await createLibraryActions(deps).cancelScan();
    expect(api.cancelScan).toHaveBeenCalled();

    vi.mocked(api.startScan).mockRejectedValue(new Error("scan fail"));
    vi.mocked(api.addSmbSource).mockResolvedValue({
      id: 1,
      path: "/s",
      kind: "smb",
    } as never);
    await createLibraryActions(deps).addMountedSmbPath("/s");
  });

  it("closes inspector in full view without clearing selection", () => {
    const deps = makeLibraryActionsDeps({ fullView: true });
    createLibraryActions(deps).closeInspector();
    createLibraryActions(deps).closeDetail();
    expect(deps.setInspectorVisible).toHaveBeenCalledWith(false);
    expect(deps.setSelectedId).not.toHaveBeenCalled();
  });

  it("handles smb connect failures and scan errors", async () => {
    vi.mocked(api.connectSmbShare).mockRejectedValue(new Error("connect fail"));
    const deps = makeLibraryActionsDeps();
    await createLibraryActions(deps).connectSmbShare({
      host: "h",
      share: "s",
      username: "u",
      password: "p",
      pollSecs: 60,
      folderPath: "/x",
    });
    expect(deps.setNotification).toHaveBeenCalled();

    vi.mocked(api.connectSmbShare).mockResolvedValue({
      id: 9,
      path: "/smb",
      kind: "smb",
    } as never);
    vi.mocked(api.startScan).mockRejectedValue(new Error("scan fail"));
    await createLibraryActions(deps).connectSmbShare({
      host: "h",
      share: "s",
      username: "u",
      password: "p",
      pollSecs: 60,
      folderPath: "/x",
    });

    vi.mocked(api.startScan).mockRejectedValue(new Error("sync fail"));
    await createLibraryActions(deps).syncRoot(3);
  });

  it("supports range selection and opening full view from first item", async () => {
    const deps = makeLibraryActionsDeps({
      selectedId: 1,
      selectedIds: new Set([1]),
      lastSelectedIndexRef: { current: 0 },
    });
    const actions = createLibraryActions(deps);
    await actions.selectAsset(sampleActionItem2, false, true);
    expect(deps.setSelectedIds).toHaveBeenCalled();

    const emptySelection = makeLibraryActionsDeps({
      selectedId: null,
      selectedIds: new Set(),
      selectedList: [],
    });
    await createLibraryActions(emptySelection).openFullView();
    expect(emptySelection.setFullView).toHaveBeenCalledWith(true);
  });

  it("purges multiple files and restores by selected id", async () => {
    const deps = makeLibraryActionsDeps({
      purgeTargetIds: [1, 2],
      selectedList: [],
      selectedId: 1,
    });
    await createLibraryActions(deps).submitPurge();
    expect(api.purgeDelete).toHaveBeenCalled();

    const restoreDeps = makeLibraryActionsDeps({
      selectedList: [],
      selectedId: 2,
    });
    await createLibraryActions(restoreDeps).restoreSelected();
    expect(api.restoreAssets).toHaveBeenCalledWith([2]);
  });

  it("creates albums and runs collection delete confirm", async () => {
    vi.mocked(api.createAlbum).mockResolvedValue({
      id: 12,
      name: "a",
      emoji: null,
      is_smart: false,
    });
    const noSelection = makeLibraryActionsDeps({
      selectedList: [],
      selectedId: null,
      selectedIds: new Set(),
    });
    await createLibraryActions(noSelection).createAlbum("solo");
    expect(api.setAlbumItems).not.toHaveBeenCalled();

    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    await actions.selectAlbum(4);
    actions.selectCollection({
      id: 2,
      name: "c",
      filter: { tag_ids: [1] },
    } as never);
    actions.deleteCollection(2);
    const [, , onConfirm] = deps.requestConfirm.mock.calls.at(-1)!;
    await onConfirm();
    expect(api.deleteSmartCollection).toHaveBeenCalledWith(2);
  });

  it("confirms relink, tag delete, and catalog rebuild", async () => {
    vi.mocked(pickFolder).mockResolvedValue("/new");
    vi.mocked(api.previewRelink).mockResolvedValue({
      matched: 2,
      total_sampled: 2,
      new_path: "/new",
    });
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    await actions.relinkRoot(1);
    const [, , relinkConfirm] = deps.requestConfirm.mock.calls.at(-1)!;
    await relinkConfirm();
    expect(api.relinkRoot).toHaveBeenCalledWith(1, "/new");

    actions.deleteTag(3);
    const [, , deleteTagConfirm] = deps.requestConfirm.mock.calls.at(-1)!;
    await deleteTagConfirm();
    expect(api.deleteTag).toHaveBeenCalledWith(3);

    actions.rebuildCatalog();
    const [, , rebuildConfirm] = deps.requestConfirm.mock.calls.at(-1)!;
    await rebuildConfirm();
    expect(api.rebuildCatalog).toHaveBeenCalled();
  });

  it("opens gallery without view transition and navigates grid", () => {
    const deps = makeLibraryActionsDeps({ selectedId: 2 });
    const actions = createLibraryActions(deps);
    const original = document.startViewTransition;
    document.startViewTransition = undefined;
    actions.openGallery();
    document.startViewTransition = original;

    actions.viewTrash();
    actions.filterByTag(9);
    actions.navigateGrid(1, 0);
    actions.navigateRelative(-1);
    expect(deps.setFilterBar).toHaveBeenCalled();
    expect(deps.setSelectedId).toHaveBeenCalled();
  });

  it("toggles fullscreen and navigates from empty selection", () => {
    const deps = makeLibraryActionsDeps({
      selectedId: null,
      selectedIds: new Set(),
      selectedList: [],
    });
    const actions = createLibraryActions(deps);
    actions.toggleFullscreen();
    expect(deps.setFullView).toHaveBeenCalledWith(true);

    const fullViewDeps = makeLibraryActionsDeps({ fullView: true });
    createLibraryActions(fullViewDeps).toggleFullscreen();
    expect(fullViewDeps.setFullView).toHaveBeenCalledWith(false);
  });

  it("does not enter fullscreen without selection or items", () => {
    const deps = makeLibraryActionsDeps({
      selectedId: null,
      selectedIds: new Set(),
      selectedList: [],
      items: [],
    });
    createLibraryActions(deps).toggleFullscreen();
    expect(deps.setFullView).not.toHaveBeenCalled();
  });

  it("updates compare details and deletes compare assets", async () => {
    vi.mocked(runAlbumToggle).mockResolvedValue(0);
    const deps = makeLibraryActionsDeps({
      compareOpen: true,
      compareItems: [sampleActionItem, sampleActionItem2],
      compareDetails: {
        1: { tag_ids: [1], album_ids: [] },
        2: { tag_ids: [], album_ids: [] },
      },
      stampArmed: true,
      stampConfig: { ...EMPTY_STAMP_CONFIG, rating: 4 },
    });
    const actions = createLibraryActions(deps);
    await actions.createTag("child", 1, "#abc");
    await actions.toggleAlbumOnSelection(2, false);
    await actions.toggleTagOnAsset(1, 2, true);
    await actions.createTagOnAsset(1, "new");
    await actions.toggleAlbumOnAsset(1, 3, true);
    await actions.createAlbumOnAsset(1, "album");
    await actions.toggleStampOnTargets();
    expect(api.createTag).toHaveBeenCalled();
    expect(runStampToggle).toHaveBeenCalled();

    vi.mocked(api.getAsset).mockResolvedValue({
      ...sampleActionDetail,
      asset: { ...sampleActionDetail.asset, id: 2 },
    });
    deps.setCompareIds = vi.fn((updater) => {
      if (typeof updater === "function") updater([1, 2]);
    });
    await actions.deleteAsset(2);
    expect(api.softDeleteAssets).toHaveBeenCalledWith([2]);
  });

  it("skips empty batch operations", async () => {
    const deps = makeLibraryActionsDeps({ selectedList: [] });
    const actions = createLibraryActions(deps);
    await actions.batchRate(3);
    await actions.createTagOnSelection("   ");
    await actions.createAlbumOnSelection("   ");
    await actions.submitSaveCollection("  ");
    expect(api.batchUpdateAssetMeta).not.toHaveBeenCalled();
  });

  it("returns early from guard paths and zero-id mutations", async () => {
    const deps = makeLibraryActionsDeps({
      selectedId: null,
      selectedIds: new Set(),
      selectedList: [],
      purgeTargetIds: [],
    });
    const actions = createLibraryActions(deps);
    await actions.rate(3);
    await actions.softDelete();
    actions.purge();
    actions.batchPurge();
    await actions.submitPurge();
    await actions.restoreSelected();
    await actions.createAlbum("  ");
    await actions.toggleTagOnSelection(1, true);
    await actions.toggleAlbumOnSelection(1, true);
    await actions.batchRemove();
    await actions.batchRate(1);
    await actions.createTagOnAsset(1, "  ");
    await actions.createAlbumOnAsset(1, "  ");
    vi.mocked(runTagCreate).mockResolvedValueOnce(0);
    expect(await actions.createTagOnSelection("tag")).toBeUndefined();
    vi.mocked(runAlbumCreate).mockResolvedValueOnce(0);
    expect(await actions.createAlbumOnSelection("album")).toBeUndefined();
    expect(api.updateAssetMeta).not.toHaveBeenCalled();
    expect(api.batchUpdateAssetMeta).not.toHaveBeenCalled();
  });

  it("focuses another selected asset after multi deselect", async () => {
    const deps = makeLibraryActionsDeps({
      selectedId: 1,
      selectedIds: new Set([1, 2]),
      selectedList: [1, 2],
      lastSelectedIndexRef: { current: 0 },
    });
    await createLibraryActions(deps).selectAsset(
      sampleActionItem2,
      true,
      false,
    );
    expect(deps.setSelectedId).toHaveBeenCalledWith(1);
  });

  it("notifies for single purge and creates album for selected id only", async () => {
    vi.mocked(api.createAlbum).mockResolvedValue({
      id: 8,
      name: "a",
      emoji: null,
      is_smart: false,
    });
    const setFilterBar = vi.fn((updater) => {
      if (typeof updater === "function") {
        updater({
          sort: "date",
          sortDir: "desc",
          tagIds: [],
          albumIds: [],
          deleteStatus: null,
        } as never);
      }
    });
    const singlePurge = makeLibraryActionsDeps({
      purgeTargetIds: [1],
      setFilterBar,
    });
    await createLibraryActions(singlePurge).submitPurge();
    expect(api.purgeDelete).toHaveBeenCalledWith([1], "DELETE");

    const albumDeps = makeLibraryActionsDeps({
      selectedList: [],
      selectedId: 1,
      setFilterBar,
    });
    await createLibraryActions(albumDeps).createAlbum("solo");
    expect(api.setAlbumItems).toHaveBeenCalledWith(8, [1]);
  });

  it("notifies album removal and runs filter bar updaters", async () => {
    const setFilterBar = vi.fn((updater) => {
      if (typeof updater === "function") {
        updater({
          sort: "date",
          sortDir: "desc",
          tagIds: [3, 4],
          albumIds: [9],
          deleteStatus: null,
        } as never);
      }
    });
    const deps = makeLibraryActionsDeps({ setFilterBar });
    const actions = createLibraryActions(deps);
    await actions.toggleAlbumOnSelection(2, false);
    await actions.selectAlbum(4);
    actions.selectCollection({
      id: 2,
      name: "c",
      filter: { tag_ids: [1] },
    } as never);
    actions.filterByTag(9);
    actions.viewTrash();
    await actions.createTag("top-level");
    actions.deleteTag(99);
    const [, , deleteTagConfirm] = deps.requestConfirm.mock.calls.at(-1)!;
    await deleteTagConfirm();
    expect(deps.setNotification).toHaveBeenCalled();
    expect(setFilterBar).toHaveBeenCalled();
  });

  it("opens gallery for selected and missing ids", () => {
    const deps = makeLibraryActionsDeps({ selectedId: 2 });
    createLibraryActions(deps).openGallery();
    expect(deps.setGalleryIndex).toHaveBeenCalledWith(1);

    const missing = makeLibraryActionsDeps({ selectedId: 99 });
    createLibraryActions(missing).openGallery();
    expect(missing.setGalleryIndex).toHaveBeenCalledWith(0);
  });

  it("returns early from openFullView with empty items", async () => {
    const deps = makeLibraryActionsDeps({ selectedId: null, items: [] });
    await createLibraryActions(deps).openFullView();
    expect(deps.setFullView).not.toHaveBeenCalled();
  });

  it("opens full view when selection exists without grid items", async () => {
    const deps = makeLibraryActionsDeps({ selectedId: 1, items: [] });
    await createLibraryActions(deps).openFullView();
    expect(deps.setFullView).toHaveBeenCalledWith(true);
  });

  it("skips detail refresh after soft deleting duplicate without selection", async () => {
    const deps = makeLibraryActionsDeps({ selectedId: null });
    await createLibraryActions(deps).softDeleteDuplicate(2);
    expect(api.getAsset).not.toHaveBeenCalled();
    expect(api.softDeleteAssets).toHaveBeenCalledWith([2]);
  });

  it("skips detail refresh when creating tag outside selection", async () => {
    const deps = makeLibraryActionsDeps({
      selectedId: 1,
      selectedIds: new Set([2]),
      selectedList: [2],
    });
    await createLibraryActions(deps).createTagOnSelection("new-tag");
    expect(api.getAsset).not.toHaveBeenCalled();
  });

  it("skips selected detail refresh when stamping compare targets", async () => {
    vi.mocked(runStampToggle).mockResolvedValueOnce({
      applied: 1,
      unstamped: 0,
      stampedIds: [1, 2],
      unstampedIds: [],
    });
    const deps = makeLibraryActionsDeps({
      compareOpen: true,
      compareItems: [sampleActionItem, sampleActionItem2],
      selectedId: 99,
      selectedList: [1, 2],
      stampArmed: true,
      stampConfig: { ...EMPTY_STAMP_CONFIG, rating: 4 },
    });
    vi.mocked(api.getAsset).mockClear();
    await createLibraryActions(deps).toggleStampOnTargets();
    expect(api.getAsset).not.toHaveBeenCalledWith(99);
  });

  it("executes grid and compare state updaters", async () => {
    const setGridColumnCount = vi.fn((updater) => {
      if (typeof updater === "function") updater(5);
    });
    const setCompareDetails = vi.fn((updater) => {
      if (typeof updater === "function") {
        updater({ 1: { tag_ids: [1], album_ids: [2] } });
      }
    });
    const deps = makeLibraryActionsDeps({
      setGridColumnCount,
      setCompareDetails,
    });
    const actions = createLibraryActions(deps);
    actions.adjustGridSize(1);
    await actions.toggleTagOnAsset(1, 2, true);
    await actions.createTagOnAsset(1, "new");
    await actions.toggleAlbumOnAsset(1, 3, true);
    await actions.createAlbumOnAsset(1, "album");
    expect(setGridColumnCount).toHaveBeenCalled();
    expect(setCompareDetails).toHaveBeenCalled();
  });

  it("returns early from navigation when no card resolves", () => {
    const navigateSpy = vi
      .spyOn(gridNavigation, "navigateGridIndex")
      .mockReturnValue(99);
    const gridDeps = makeLibraryActionsDeps({
      selectedId: null,
      items: [sampleActionItem],
    });
    createLibraryActions(gridDeps).navigateGrid(1, 0);
    expect(gridDeps.setSelectedId).not.toHaveBeenCalled();
    navigateSpy.mockRestore();

    const emptyItems = makeLibraryActionsDeps({ items: [] });
    createLibraryActions(emptyItems).navigateGrid(1, 0);
    createLibraryActions(emptyItems).navigateRelative(1);

    const relativeDeps = makeLibraryActionsDeps({ selectedId: null });
    createLibraryActions(relativeDeps).navigateRelative(1);
    createLibraryActions(relativeDeps).navigateRelative(-1);

    const sparseItems = {
      length: 2,
      1: sampleActionItem,
    } as unknown as typeof relativeDeps.items;
    const sparseDeps = makeLibraryActionsDeps({
      selectedId: null,
      items: sparseItems,
    });
    createLibraryActions(sparseDeps).navigateRelative(1);
    expect(sparseDeps.setSelectedId).not.toHaveBeenCalled();
  });

  it("resolves stamp snapshots and refreshes compare details", async () => {
    vi.mocked(runStampToggle).mockImplementation(
      async (targets, _config, loadSnapshot) => {
        for (const id of targets) {
          await loadSnapshot(id);
        }
        return {
          applied: 1,
          unstamped: 0,
          stampedIds: targets,
          unstampedIds: [],
        };
      },
    );

    const compareDeps = makeLibraryActionsDeps({
      compareOpen: true,
      compareItems: [sampleActionItem, sampleActionItem2],
      compareDetails: {
        1: { tag_ids: [1], album_ids: [2] },
        2: { tag_ids: [], album_ids: [] },
      },
      selectedList: [1, 2],
      stampArmed: true,
      stampConfig: { ...EMPTY_STAMP_CONFIG, rating: 4 },
      setCompareDetails: vi.fn((updater) => {
        if (typeof updater === "function") {
          updater({
            1: { tag_ids: [1], album_ids: [2] },
            2: { tag_ids: [], album_ids: [] },
          });
        }
      }),
    });
    await createLibraryActions(compareDeps).toggleStampOnTargets();
    expect(runStampToggle).toHaveBeenCalled();

    const detailDeps = makeLibraryActionsDeps({
      fullView: true,
      compareOpen: false,
      selectedId: 1,
      detail: sampleActionDetail,
      stampArmed: true,
      stampConfig: { ...EMPTY_STAMP_CONFIG, rating: 2 },
    });
    await createLibraryActions(detailDeps).toggleStampOnTargets();

    vi.mocked(api.getAsset).mockResolvedValueOnce({
      ...sampleActionDetail,
      asset: { ...sampleActionDetail.asset, id: 99 },
    });
    const fallbackDeps = makeLibraryActionsDeps({
      selectedId: 99,
      selectedList: [99],
      detail: sampleActionDetail,
      stampArmed: true,
      stampConfig: { ...EMPTY_STAMP_CONFIG, rating: 1 },
    });
    await createLibraryActions(fallbackDeps).toggleStampOnTargets();
    expect(api.getAsset).toHaveBeenCalledWith(99);
  });

  it("keeps compare open and clears selection on deleteAsset", async () => {
    const item3 = { ...sampleActionItem, id: 3, file_name: "photo3.jpg" };
    const setCompareIds = vi.fn((updater) => {
      if (typeof updater === "function") updater([1, 2, 3]);
    });
    const setCompareItems = vi.fn((updater) => {
      if (typeof updater === "function") {
        updater([sampleActionItem, sampleActionItem2, item3]);
      }
    });
    const setCompareDetails = vi.fn((updater) => {
      if (typeof updater === "function") {
        updater({
          1: { tag_ids: [], album_ids: [] },
          2: { tag_ids: [], album_ids: [] },
          3: { tag_ids: [], album_ids: [] },
        });
      }
    });
    const setSelectedIds = vi.fn((updater) => {
      if (typeof updater === "function") updater(new Set([1, 2, 3]));
    });
    const deps = makeLibraryActionsDeps({
      selectedId: 2,
      compareOpen: true,
      compareItems: [sampleActionItem, sampleActionItem2, item3],
      setCompareIds,
      setCompareItems,
      setCompareDetails,
      setSelectedIds,
    });
    await createLibraryActions(deps).deleteAsset(2);
    expect(setCompareIds).toHaveBeenCalled();
    expect(deps.setSelectedId).toHaveBeenCalledWith(null);
    expect(deps.setDetail).toHaveBeenCalledWith(null);
  });

  it("clears selected collection when deleting active collection", async () => {
    const setSelectedCollectionId = vi.fn((updater) => {
      if (typeof updater === "function") updater(3);
    });
    const deps = makeLibraryActionsDeps({ setSelectedCollectionId });
    createLibraryActions(deps).deleteCollection(3);
    const [, , onConfirm] = deps.requestConfirm.mock.calls.at(-1)!;
    await onConfirm();
    expect(setSelectedCollectionId).toHaveBeenCalled();
  });

  it("opens gallery from start when nothing is selected", () => {
    const deps = makeLibraryActionsDeps({ selectedId: null });
    createLibraryActions(deps).openGallery();
    expect(deps.setGalleryIndex).toHaveBeenCalledWith(0);
  });

  it("closes compare when delete leaves fewer than two assets", async () => {
    const setCompareIds = vi.fn((updater) => {
      if (typeof updater === "function") updater([1, 2]);
    });
    const deps = makeLibraryActionsDeps({
      compareOpen: true,
      compareItems: [sampleActionItem, sampleActionItem2],
      setCompareIds,
    });
    await createLibraryActions(deps).deleteAsset(2);
    expect(deps.setCompareOpen).toHaveBeenCalledWith(false);
    expect(deps.setCompareItems).toHaveBeenCalledWith([]);
    expect(deps.setCompareDetails).toHaveBeenCalledWith({});
  });

  it("skips detail refresh when selected id is outside selection", async () => {
    const deps = makeLibraryActionsDeps({
      selectedId: 99,
      selectedIds: new Set([1]),
      selectedList: [1],
    });
    const actions = createLibraryActions(deps);
    await actions.toggleTagOnSelection(3, true);
    await actions.toggleAlbumOnSelection(4, true);
    expect(api.getAsset).not.toHaveBeenCalled();
  });

  it("returns undefined when tag or album create yields zero id", async () => {
    vi.mocked(runTagCreate).mockResolvedValue(0);
    vi.mocked(runAlbumCreate).mockResolvedValue(0);
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    expect(await actions.createTagOnSelection("tag")).toBeUndefined();
    expect(await actions.createTagOnAsset(1, "tag")).toBeUndefined();
    expect(await actions.createAlbumOnSelection("album")).toBeUndefined();
    expect(await actions.createAlbumOnAsset(1, "album")).toBeUndefined();
  });

  it("keeps selected collection when deleting another collection", async () => {
    const setSelectedCollectionId = vi.fn((updater) => {
      if (typeof updater === "function") updater(5);
    });
    const deps = makeLibraryActionsDeps({ setSelectedCollectionId });
    createLibraryActions(deps).deleteCollection(3);
    const [, , onConfirm] = deps.requestConfirm.mock.calls.at(-1)!;
    await onConfirm();
    expect(setSelectedCollectionId).toHaveBeenCalled();
  });

  it("blocks delete when tag or album is referenced by stamp", () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps);
    actions.deleteAlbum(8);
    actions.deleteTag(7);
    expect(deps.requestConfirm).not.toHaveBeenCalled();
    expect(deps.setNotification).toHaveBeenCalledTimes(2);
  });

  it("runs delete confirm callbacks for album and tag", async () => {
    const setFilterBar = vi.fn((updater) => {
      if (typeof updater === "function") {
        updater({
          sort: "date",
          sortDir: "desc",
          tagIds: [99],
          albumIds: [],
          deleteStatus: null,
        } as never);
      }
    });
    const deps = makeLibraryActionsDeps({ setFilterBar });
    const actions = createLibraryActions(deps);
    actions.deleteAlbum(1);
    await deps.requestConfirm.mock.calls.at(-1)![2]();
    actions.deleteTag(99);
    await deps.requestConfirm.mock.calls.at(-1)![2]();
    expect(api.deleteAlbum).toHaveBeenCalledWith(1);
    expect(api.deleteTag).toHaveBeenCalledWith(99);
    expect(setFilterBar).toHaveBeenCalled();
  });
});
