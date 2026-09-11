import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { EMPTY_STAMP_CONFIG } from "./lib/stamp";

const useWorkspaceMock = vi.hoisted(() => vi.fn());

vi.mock("./hooks/useWorkspace", () => ({
  useWorkspace: () => useWorkspaceMock(),
}));

vi.mock("./hooks/useLibrary", () => ({
  useLibrary: () => ({
    roots: [],
    albums: [],
    collections: [],
    tags: [],
    deletedCount: 0,
    items: [],
    total: 0,
    detail: null,
    busy: false,
    notification: null,
    scanStatus: "",
    galleryIndex: null,
    setGalleryIndex: vi.fn(),
    fullView: false,
    gridColumnCount: 5,
    setGridColumnCount: vi.fn(),
    compareOpen: false,
    compareItems: [],
    compareDetails: {},
    inspectorVisible: true,
    smbDialogOpen: false,
    setSmbDialogOpen: vi.fn(),
    collectionDialogOpen: false,
    closeCollectionDialog: vi.fn(),
    tagMenuOpen: false,
    setTagMenuOpen: vi.fn(),
    albumMenuOpen: false,
    setAlbumMenuOpen: vi.fn(),
    purgeDialogOpen: false,
    purgeTargetIds: [],
    closePurgeDialog: vi.fn(),
    loadingMore: false,
    hasMore: false,
    filterBar: {
      ratingMin: "",
      syncStates: [],
      deleteStatus: "",
      camera: "",
      tagIds: [],
      albumIds: [],
      metaSearch: "",
      hasGps: false,
      hasDuplicate: false,
      captureFrom: "",
      captureTo: "",
      sort: "date",
      sortDir: "desc",
    },
    extraFilter: {},
    selectedCollectionId: null,
    setFilterBar: vi.fn(),
    selectedId: null,
    selectedIds: new Set<number>(),
    actions: {
      addLocalRoot: vi.fn(),
      addMountedSmbPath: vi.fn(),
      openSmbConnect: vi.fn(),
      connectSmbShare: vi.fn(),
      removeRoot: vi.fn(),
      syncRoot: vi.fn(),
      selectRoot: vi.fn(),
      clearFilters: vi.fn(),
      openExport: vi.fn(),
      createAlbum: vi.fn(),
      deleteAlbum: vi.fn(),
      selectAlbum: vi.fn(),
      selectCollection: vi.fn(),
      deleteCollection: vi.fn(),
      relinkRoot: vi.fn(),
      selectAsset: vi.fn(),
      openLinked: vi.fn(),
      rate: vi.fn(),
      batchRate: vi.fn(),
      openTagMenu: vi.fn(),
      openAlbumMenu: vi.fn(),
      toggleTagOnSelection: vi.fn(),
      createTagOnSelection: vi.fn(),
      toggleAlbumOnSelection: vi.fn(),
      createAlbumOnSelection: vi.fn(),
      batchRemove: vi.fn(),
      batchPurge: vi.fn(),
      softDeleteDuplicate: vi.fn(),
      softDelete: vi.fn(),
      purge: vi.fn(),
      submitPurge: vi.fn(),
      restoreSelected: vi.fn(),
      saveCollection: vi.fn(),
      submitSaveCollection: vi.fn(),
      openGallery: vi.fn(),
      closeDetail: vi.fn(),
      closeInspector: vi.fn(),
      clearSelection: vi.fn(),
      loadMore: vi.fn(),
      filterByTag: vi.fn(),
      createTag: vi.fn(),
      updateTag: vi.fn(),
      deleteTag: vi.fn(),
      updateAlbum: vi.fn(),
      viewTrash: vi.fn(),
      viewLibrary: vi.fn(),
      rebuildCatalog: vi.fn(),
      cancelScan: vi.fn(),
      navigateRelative: vi.fn(),
      navigateGrid: vi.fn(),
      adjustGridSize: vi.fn(),
      openFullView: vi.fn(),
      openFullViewFor: vi.fn(),
      closeFullView: vi.fn(),
      toggleFullscreen: vi.fn(),
      openCompare: vi.fn(),
      closeCompare: vi.fn(),
      rateAsset: vi.fn(),
      toggleTagOnAsset: vi.fn(),
      createTagOnAsset: vi.fn(),
      toggleAlbumOnAsset: vi.fn(),
      createAlbumOnAsset: vi.fn(),
      deleteAsset: vi.fn(),
      disarmStamp: vi.fn(),
      setStampRating: vi.fn(),
      toggleStampTag: vi.fn(),
      toggleStampAlbum: vi.fn(),
      toggleStampOnTargets: vi.fn(),
    },
    stampConfig: EMPTY_STAMP_CONFIG,
    stampArmed: false,
    stampMatchedIds: new Set<number>(),
    exportDialog: {
      open: false,
      assetIds: [],
      destination: "",
      options: { flat: true, rename_template: undefined, format: undefined },
      jobId: null,
      progress: null,
    },
    closeExportDialog: vi.fn(),
    updateExportDialog: vi.fn(),
    startExportFromDialog: vi.fn(),
    confirmDialog: { open: false, title: "", message: "", onConfirm: vi.fn() },
    closeConfirmDialog: vi.fn(),
    dismissToast: vi.fn(),
  }),
}));

describe("App", () => {
  it("renders loading spinner while workspace bootstraps", () => {
    useWorkspaceMock.mockReturnValue({
      phase: "loading",
      workspace: null,
      recent: [],
      busy: false,
      busyMessage: "",
      notification: null,
      dismissToast: vi.fn(),
      pickAndCreateWorkspace: vi.fn(),
      pickAndOpenWorkspace: vi.fn(),
      openWorkspacePath: vi.fn(),
      closeWorkspace: vi.fn(),
      removeRecent: vi.fn(),
    });
    const { container } = render(<App />);
    expect(container.querySelector(".loading-spinner")).toBeInTheDocument();
  });

  it("renders library shell when workspace is open", async () => {
    useWorkspaceMock.mockReturnValue({
      phase: "library",
      workspace: { path: "/tmp/ws", name: "Demo", id: "ws-1", read_only: false },
      recent: [],
      busy: false,
      busyMessage: "",
      notification: null,
      dismissToast: vi.fn(),
      pickAndCreateWorkspace: vi.fn(),
      pickAndOpenWorkspace: vi.fn(),
      openWorkspacePath: vi.fn(),
      closeWorkspace: vi.fn(),
      removeRecent: vi.fn(),
    });
    render(<App />);
    expect(await screen.findByTitle("Library")).toBeInTheDocument();
    expect(screen.getByTitle("Close workspace")).toBeInTheDocument();
    expect(screen.getByText("Ready")).toBeInTheDocument();
  });

  it("renders start page when no workspace is open", () => {
    useWorkspaceMock.mockReturnValue({
      phase: "start",
      workspace: null,
      recent: [{ path: "/tmp/ws", name: "Demo", last_opened_at: 1 }],
      busy: false,
      busyMessage: "",
      notification: null,
      dismissToast: vi.fn(),
      pickAndCreateWorkspace: vi.fn(),
      pickAndOpenWorkspace: vi.fn(),
      openWorkspacePath: vi.fn(),
      closeWorkspace: vi.fn(),
      removeRecent: vi.fn(),
    });
    render(<App />);
    expect(screen.getByText("Open Workspace")).toBeInTheDocument();
  });

  it("closes workspace from library shell", async () => {
    const closeWorkspace = vi.fn();
    useWorkspaceMock.mockReturnValue({
      phase: "library",
      workspace: { path: "/tmp/ws", name: "Demo", id: "ws-1", read_only: false },
      recent: [],
      busy: false,
      busyMessage: "",
      notification: null,
      dismissToast: vi.fn(),
      pickAndCreateWorkspace: vi.fn(),
      pickAndOpenWorkspace: vi.fn(),
      openWorkspacePath: vi.fn(),
      closeWorkspace,
      removeRecent: vi.fn(),
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByTitle("Close workspace"));
    expect(closeWorkspace).toHaveBeenCalled();
  });
});
