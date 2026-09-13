import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as api from "../api/client";
import {
  handleGridCardClick,
  resetGridCardClickState,
} from "../lib/gridCardClick";
import { EMPTY_STAMP_CONFIG } from "../lib/stamp";
import {
  emptyFilterBar,
  mockLibraryActions,
  sampleCard,
  sampleDetail,
  sampleVideoCard,
} from "../test/fixtures";
import { LibraryApp } from "./LibraryApp";

const { useLibraryMock, gridSampleCard, pickFolder } = vi.hoisted(() => ({
  useLibraryMock: vi.fn(),
  pickFolder: vi.fn(),
  gridSampleCard: {
    id: 1,
    file_name: "photo.jpg",
    ext: "jpg",
    kind: "image" as const,
    capture_at: null,
    rating: null,
    sync_state: "ok" as const,
    thumb_path: "/tmp/1.webp",
    abs_path: "/tmp/1.jpg",
    has_duplicate: false,
  },
}));

vi.mock("../hooks/useLibrary", () => ({
  useLibrary: () => useLibraryMock(),
}));

vi.mock("./VirtualGrid", () => ({
  VirtualGrid: ({
    onSelect,
    onOpenFullView,
    onLoadMore,
  }: {
    onSelect: (
      card: typeof gridSampleCard,
      multi: boolean,
      range: boolean,
    ) => void;
    onOpenFullView: (card: typeof gridSampleCard) => void;
    onLoadMore?: () => void;
  }) => (
    <div>
      <button
        type="button"
        className="aspect-square"
        data-load-more=""
        onClick={() => onLoadMore?.()}
      >
        grid
      </button>
      <button
        type="button"
        className="aspect-square"
        aria-label="card"
        onClick={() =>
          handleGridCardClick(
            gridSampleCard.id,
            () => onSelect(gridSampleCard, false, false),
            () => onOpenFullView(gridSampleCard),
          )
        }
      >
        card
      </button>
    </div>
  ),
}));

vi.mock("../hooks/useReducedMotion", () => ({
  useReducedMotion: () => false,
}));

vi.mock("../lib/windowFullscreen", () => ({
  exitWindowFullscreen: vi.fn(async () => undefined),
}));

vi.mock("../hooks/useSlideshowFullscreen", () => ({
  useSlideshowFullscreen: vi.fn(),
}));

vi.mock("../lib/pickFolder", () => ({
  pickFolder,
}));

vi.mock("../api/client", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../api/client")>();
  return {
    ...actual,
    queryAssetActivity: vi.fn().mockResolvedValue([]),
    listSmbShares: vi.fn().mockResolvedValue([]),
    mountSmbForBrowse: vi.fn(),
    listFolderChildren: vi.fn().mockResolvedValue([]),
  };
});

vi.mock("../hooks/useSlideshow", () => ({
  useSlideshow: () => ({
    settings: {
      intervalMs: 3000,
      theme: "dissolve",
      loop: true,
      shuffle: false,
      muteVideos: false,
    },
    playing: true,
    setPlaying: vi.fn(),
    theme: "dissolve",
    fromIndex: null,
    toIndex: 0,
    progress: 1,
    incomingElapsedMs: 0,
    outgoingElapsedMs: 0,
    goNext: vi.fn(),
    goPrev: vi.fn(),
    onVideoEnded: vi.fn(),
    cycleInterval: vi.fn(),
    cycleTheme: vi.fn(),
    toggleLoop: vi.fn(),
    toggleShuffle: vi.fn(),
    toggleMute: vi.fn(),
  }),
}));

function createMockLibrary(overrides: Record<string, unknown> = {}) {
  const actions = mockLibraryActions();
  return {
    roots: [],
    albums: [],
    collections: [],
    tags: [{ id: 1, name: "trip", parent_id: null, color: null }],
    deletedCount: 0,
    items: [sampleCard, sampleVideoCard],
    total: 2,
    detail: null,
    busy: false,
    notification: null,
    scanStatus: "",
    scanStatusByRoot: {},
    galleryIndex: null,
    setGalleryIndex: vi.fn(),
    fullView: false,
    inspectorVisible: true,
    gridColumnCount: 5,
    setGridColumnCount: vi.fn(),
    compareOpen: false,
    compareItems: [],
    compareDetails: {},
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
    filterBar: emptyFilterBar,
    extraFilter: {},
    selectedCollectionId: null,
    setFilterBar: vi.fn(),
    selectedId: null,
    selectedIds: new Set<number>(),
    actions,
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
    undoActivity: vi.fn(),
    ...overrides,
  };
}

describe("LibraryApp", () => {
  afterEach(() => {
    resetGridCardClickState();
  });

  it("renders nav rail and grid area", () => {
    useLibraryMock.mockReturnValue(createMockLibrary());
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(
      screen.getByRole("button", { name: "Library" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Ready")).toBeInTheDocument();
  });

  it("calls onCloseWorkspace from nav rail", async () => {
    const onCloseWorkspace = vi.fn();
    useLibraryMock.mockReturnValue(createMockLibrary());
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={onCloseWorkspace} />);
    await user.click(screen.getByRole("button", { name: "Close workspace" }));
    expect(onCloseWorkspace).toHaveBeenCalledTimes(1);
  });

  it("shows export dialog when open", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        exportDialog: {
          open: true,
          assetIds: [1],
          destination: "",
          options: {
            flat: true,
            rename_template: undefined,
            format: undefined,
          },
          jobId: null,
          progress: null,
        },
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getByText("Export files")).toBeInTheDocument();
  });

  it("shows confirm dialog and runs confirm handler", async () => {
    const onConfirm = vi.fn().mockResolvedValue(undefined);
    const closeConfirmDialog = vi.fn();
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        confirmDialog: {
          open: true,
          title: "Remove source",
          message: "This cannot be undone.",
          onConfirm,
        },
        closeConfirmDialog,
      }),
    );
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(onConfirm).toHaveBeenCalled();
  });

  it("shows purge dialog when open", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        purgeDialogOpen: true,
        purgeTargetIds: [1],
        detail: sampleDetail,
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getByText("Purge file")).toBeInTheDocument();
  });

  it("shows smb connect dialog when open", () => {
    useLibraryMock.mockReturnValue(createMockLibrary({ smbDialogOpen: true }));
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getByText("Add network source")).toBeInTheDocument();
  });

  it("shows collection name prompt when open", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({ collectionDialogOpen: true }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(
      screen.getByRole("heading", { name: "Save as Collection" }),
    ).toBeInTheDocument();
  });

  it("renders compare viewer when compare is open", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        compareOpen: true,
        compareItems: [sampleCard, sampleVideoCard],
        compareDetails: {
          1: { tag_ids: [1], album_ids: [] },
          2: { tag_ids: [], album_ids: [] },
        },
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getByText("1 / 2 — photo.jpg")).toBeInTheDocument();
  });

  it("renders full view when enabled", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        fullView: true,
        selectedId: 1,
        selectedIds: new Set([1]),
        detail: sampleDetail,
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getAllByLabelText("Close").length).toBeGreaterThan(0);
  });

  it("renders gallery overlay", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        galleryIndex: 0,
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getAllByLabelText("Close").length).toBeGreaterThan(0);
  });

  it("shows floating selection bar for multi-select", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        selectedIds: new Set([1, 2]),
        selectedId: 1,
        detail: sampleDetail,
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(
      screen.getByRole("button", { name: /Compare/i }),
    ).toBeInTheDocument();
  });

  it("shows inspector placeholder without detail", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        selectedIds: new Set([1]),
        selectedId: 1,
        detail: null,
      }),
    );
    const { container } = render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(container.querySelector(".loading-spinner")).toBeInTheDocument();
  });

  it("shows inspector panel with detail", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        selectedIds: new Set([1]),
        selectedId: 1,
        detail: sampleDetail,
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getAllByText("photo.jpg").length).toBeGreaterThan(0);
  });

  it("shows trash floating bar mode", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        selectedIds: new Set([1]),
        selectedId: 1,
        detail: sampleDetail,
        filterBar: { ...emptyFilterBar, deleteStatus: "deleted" },
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(
      screen.getByRole("button", { name: /^Restore/i }),
    ).toBeInTheDocument();
  });

  it("passes export toolbar action for all items when nothing selected", async () => {
    const library = createMockLibrary();
    useLibraryMock.mockReturnValue(library);
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Export" }));
    expect(library.actions.openExport).toHaveBeenCalledWith([1, 2]);
  });

  it("exports only selected ids from toolbar when selection exists", async () => {
    const library = createMockLibrary({
      selectedIds: new Set([1]),
      selectedId: 1,
    });
    useLibraryMock.mockReturnValue(library);
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Export" }));
    expect(library.actions.openExport).toHaveBeenCalledWith([1]);
  });

  it("wires compare viewer actions to library actions", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      compareOpen: true,
      compareItems: [sampleCard, sampleVideoCard],
      compareDetails: {
        1: { tag_ids: [1], album_ids: [] },
        2: { tag_ids: [], album_ids: [] },
      },
      stampArmed: true,
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    window.dispatchEvent(
      new KeyboardEvent("keydown", { key: " ", bubbles: true }),
    );
    expect(library.actions.toggleStampOnTargets).toHaveBeenCalled();
    const compareBar = screen.getAllByRole("toolbar", {
      name: "Item actions",
    })[0];
    await user.click(
      within(compareBar).getByRole("button", { name: "Export" }),
    );
    expect(library.actions.openExport).toHaveBeenCalledWith([1]);
    await user.click(
      within(compareBar).getByRole("button", { name: "Delete" }),
    );
    expect(library.actions.deleteAsset).toHaveBeenCalledWith(1);
    await user.keyboard("{Escape}");
    expect(library.actions.closeCompare).toHaveBeenCalled();
    await user.click(screen.getAllByLabelText("Rotate clockwise")[0]);
    expect(library.actions.rotateAsset).toHaveBeenCalledWith(1, "cw");
  });

  it("wires full view rotation actions", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      fullView: true,
      selectedId: 1,
      selectedIds: new Set([1]),
      detail: sampleDetail,
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByLabelText("Rotate clockwise"));
    expect(library.actions.rotate).toHaveBeenCalledWith("cw");
    await user.click(screen.getByLabelText("Rotate counter-clockwise"));
    expect(library.actions.rotate).toHaveBeenCalledWith("ccw");
  });

  it("wires full view navigation and inspector actions", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      fullView: true,
      selectedId: 1,
      selectedIds: new Set([1]),
      detail: sampleDetail,
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "×" }));
    expect(library.actions.closeInspector).toHaveBeenCalled();
  });

  it("wires floating selection bar actions", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      selectedIds: new Set([1, 2]),
      selectedId: 1,
      detail: sampleDetail,
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    const selectionBar = screen.getByRole("toolbar", {
      name: "Selection actions",
    });
    await user.click(
      within(selectionBar).getByRole("button", { name: /^Export/i }),
    );
    expect(library.actions.openExport).toHaveBeenCalledWith([1, 2]);
    await user.click(
      within(selectionBar).getByRole("button", { name: /^Delete/i }),
    );
    expect(library.actions.batchRemove).toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Clear selection" }));
    expect(library.actions.clearSelection).toHaveBeenCalled();
  });

  it("wires trash floating bar restore and purge", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      selectedIds: new Set([1]),
      selectedId: 1,
      detail: sampleDetail,
      filterBar: { ...emptyFilterBar, deleteStatus: "deleted" },
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    const selectionBar = screen.getByRole("toolbar", {
      name: "Selection actions",
    });
    await user.click(
      within(selectionBar).getByRole("button", { name: /^Restore/i }),
    );
    expect(library.actions.restoreSelected).toHaveBeenCalled();
    await user.click(
      within(selectionBar).getByRole("button", { name: /^Purge/i }),
    );
    expect(library.actions.batchPurge).toHaveBeenCalled();
  });

  it("submits collection name prompt", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({ collectionDialogOpen: true });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.type(screen.getByRole("textbox"), "Favorites");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(library.actions.submitSaveCollection).toHaveBeenCalledWith(
      "Favorites",
    );
  });

  it("confirms purge dialog", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      purgeDialogOpen: true,
      purgeTargetIds: [1],
      detail: sampleDetail,
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.type(screen.getByRole("textbox"), "DELETE");
    await user.click(screen.getByRole("button", { name: "Purge" }));
    expect(library.actions.submitPurge).toHaveBeenCalled();
  });

  it("hides purge UI in read-only workspaces", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        selectedIds: new Set([1]),
        selectedId: 1,
        detail: sampleDetail,
        filterBar: { ...emptyFilterBar, deleteStatus: "deleted" },
        purgeDialogOpen: true,
        purgeTargetIds: [1],
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} readOnly />);
    expect(
      screen.queryByRole("button", { name: /^Purge/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Purge" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("Purge file")).not.toBeInTheDocument();
  });

  it("starts export from dialog", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      exportDialog: {
        open: true,
        assetIds: [1],
        destination: "/tmp/out",
        options: { flat: true, rename_template: undefined, format: undefined },
        jobId: null,
        progress: null,
      },
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    const exportDialog = screen.getByRole("dialog");
    await user.click(
      within(exportDialog).getByRole("button", { name: "Export" }),
    );
    expect(library.startExportFromDialog).toHaveBeenCalled();
  });

  it("rates from gallery overlay", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({ galleryIndex: 0 });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.keyboard("3");
    expect(library.actions.rateAsset).toHaveBeenCalledWith(1, 3);
  });

  it("shows error notification in status bar without dismiss control", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        notification: { kind: "error", text: "disk full" },
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getByText("disk full")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Close" }),
    ).not.toBeInTheDocument();
  });

  it("opens slideshow from toolbar", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary();
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Slideshow" }));
    expect(library.actions.openGallery).toHaveBeenCalled();
  });

  it("exports selected id when nothing is in the selection set", async () => {
    const library = createMockLibrary({
      selectedId: 1,
      selectedIds: new Set<number>(),
    });
    useLibraryMock.mockReturnValue(library);
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Export" }));
    expect(library.actions.openExport).toHaveBeenCalledWith([1]);
  });

  it("resizes panels and closes overlays", async () => {
    const setGalleryIndex = vi.fn();
    const setSmbDialogOpen = vi.fn();
    const library = createMockLibrary({
      galleryIndex: 0,
      setGalleryIndex,
      smbDialogOpen: true,
      setSmbDialogOpen,
      collectionDialogOpen: true,
      exportDialog: {
        open: true,
        assetIds: [1],
        destination: "/tmp/export",
        options: { flat: true, rename_template: undefined, format: undefined },
        jobId: null,
        progress: null,
      },
    });
    useLibraryMock.mockReturnValue(library);
    const user = userEvent.setup();
    const { container } = render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);

    const splitRoot = container.querySelector(
      ".flex.min-h-0.min-w-0.flex-1",
    ) as HTMLElement;
    splitRoot.getBoundingClientRect = () =>
      ({
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 1200,
        bottom: 800,
        width: 1200,
        height: 800,
        toJSON: () => ({}),
      }) as DOMRect;
    const divider = container.querySelector(
      ".cursor-col-resize",
    ) as HTMLElement;
    divider.setPointerCapture = vi.fn();
    divider.releasePointerCapture = vi.fn();
    fireEvent.pointerDown(divider, { clientX: 240, pointerId: 1 });
    fireEvent.pointerMove(window, { clientX: 320, pointerId: 1 });
    fireEvent.pointerUp(window, { pointerId: 1 });

    const galleryClose = screen
      .getAllByLabelText("Close")
      .find((button) => button.closest(".fixed.inset-0"));
    await user.click(galleryClose!);
    expect(setGalleryIndex).toHaveBeenCalledWith(null);
  });

  it("undoes inspector activity and uses tag keys for single selection", async () => {
    vi.mocked(api.queryAssetActivity).mockResolvedValue([
      {
        id: 7,
        seq: 1,
        occurred_at: 1,
        event_type: "asset.tags_added",
        actor: "user",
        correlation_id: null,
        subject_type: "asset",
        subject_id: 1,
        subject_key: null,
        summary: null,
        reversible: true,
      },
    ]);
    const library = createMockLibrary({
      selectedIds: new Set([1]),
      selectedId: 1,
      detail: sampleDetail,
    });
    useLibraryMock.mockReturnValue(library);
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Undo" })).toBeInTheDocument();
    });
    await user.click(screen.getByRole("button", { name: "Undo" }));
    expect(library.undoActivity).toHaveBeenCalledWith(7);
  });

  it("hides tag keys when detail does not match selection", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        selectedIds: new Set([1]),
        selectedId: 1,
        detail: {
          ...sampleDetail,
          asset: { ...sampleDetail.asset, id: 99 },
        },
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getAllByText("photo.jpg").length).toBeGreaterThan(0);
  });

  it("selects grid asset and opens full view", () => {
    const library = createMockLibrary();
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    const gridButton = screen.getByRole("button", { name: "card" });
    fireEvent.click(gridButton);
    expect(library.actions.selectAsset).toHaveBeenCalledWith(
      gridSampleCard,
      false,
      false,
    );
    fireEvent.click(gridButton);
    expect(library.actions.openFullViewFor).toHaveBeenCalledWith(
      gridSampleCard,
    );
  });

  it("closes full view and navigates relative", async () => {
    const library = createMockLibrary({
      fullView: true,
      selectedId: 1,
      selectedIds: new Set([1]),
      detail: sampleDetail,
    });
    useLibraryMock.mockReturnValue(library);
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByLabelText("Close"));
    expect(library.actions.closeFullView).toHaveBeenCalled();
    await user.click(screen.getByLabelText("Next"));
    expect(library.actions.navigateRelative).toHaveBeenCalledWith(1);
  });

  it("rates selection from floating bar", async () => {
    const library = createMockLibrary({
      selectedIds: new Set([1, 2]),
      selectedId: 1,
      detail: sampleDetail,
    });
    useLibraryMock.mockReturnValue(library);
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    const selectionBar = screen.getByRole("toolbar", {
      name: "Selection actions",
    });
    await user.click(
      within(selectionBar).getByRole("button", { name: "Rate 3 stars" }),
    );
    expect(library.actions.batchRate).toHaveBeenCalledWith(3);
  });

  it("closes dialogs via cancel actions", async () => {
    const closeConfirmDialog = vi.fn();
    const closeCollectionDialog = vi.fn();
    const closePurgeDialog = vi.fn();
    const closeExportDialog = vi.fn();
    const setSmbDialogOpen = vi.fn();
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        confirmDialog: {
          open: true,
          title: "Remove",
          message: "Sure?",
          onConfirm: vi.fn(),
        },
        closeConfirmDialog,
        collectionDialogOpen: true,
        closeCollectionDialog,
        purgeDialogOpen: true,
        purgeTargetIds: [1],
        detail: sampleDetail,
        closePurgeDialog,
        smbDialogOpen: true,
        setSmbDialogOpen,
        exportDialog: {
          open: true,
          assetIds: [1],
          destination: "/tmp/out",
          options: {
            flat: true,
            rename_template: undefined,
            format: undefined,
          },
          jobId: null,
          progress: null,
        },
        closeExportDialog,
      }),
    );
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    const cancelButtons = screen.getAllByRole("button", { name: "Cancel" });
    for (const button of cancelButtons) {
      await user.click(button);
    }
    expect(closeConfirmDialog).toHaveBeenCalled();
    expect(closeCollectionDialog).toHaveBeenCalled();
    expect(closePurgeDialog).toHaveBeenCalled();
    expect(setSmbDialogOpen).toHaveBeenCalledWith(false);
    expect(closeExportDialog).toHaveBeenCalled();
  });

  it("switches nav rail tab", async () => {
    useLibraryMock.mockReturnValue(createMockLibrary());
    const user = userEvent.setup();
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Collection" }));
    expect(
      screen.getByRole("button", { name: "Collection" }),
    ).toBeInTheDocument();
  });

  it("resizes leading panel width", () => {
    useLibraryMock.mockReturnValue(createMockLibrary());
    const { container } = render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    const splitRoot = container.querySelector(".flex.h-screen") as HTMLElement;
    splitRoot.getBoundingClientRect = () =>
      ({
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 1200,
        bottom: 800,
        width: 1200,
        height: 800,
        toJSON: () => ({}),
      }) as DOMRect;
    const divider = container.querySelector(
      ".cursor-col-resize",
    ) as HTMLElement;
    divider.setPointerCapture = vi.fn();
    divider.releasePointerCapture = vi.fn();
    fireEvent.pointerDown(divider, { clientX: 240, pointerId: 1 });
    fireEvent.pointerMove(window, { clientX: 320, pointerId: 1 });
    fireEvent.pointerUp(window, { pointerId: 1 });
    expect(divider).toBeInTheDocument();
  });

  it("loads more grid items when scrolled near bottom", () => {
    const library = createMockLibrary({ hasMore: true });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "grid" }));
    expect(library.actions.loadMore).toHaveBeenCalled();
  });

  it("wires compare tag and album callbacks", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      compareOpen: true,
      compareItems: [sampleCard, sampleVideoCard],
      compareDetails: {
        1: { tag_ids: [1], album_ids: [] },
        2: { tag_ids: [], album_ids: [] },
      },
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(
      screen.getAllByRole("button", { name: /Rate 2 stars/i })[0],
    );
    expect(library.actions.rateAsset).toHaveBeenCalledWith(1, 2);
    await user.click(screen.getAllByText("Tag")[0]);
    await user.click(screen.getAllByText("Album")[0]);
  });

  it("resizes inspector side panel in full view", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        fullView: true,
        selectedId: 1,
        selectedIds: new Set([1]),
        detail: sampleDetail,
      }),
    );
    const { container } = render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    const dividers = container.querySelectorAll(".cursor-col-resize");
    const divider = dividers[dividers.length - 1] as HTMLElement;
    divider.setPointerCapture = vi.fn();
    divider.releasePointerCapture = vi.fn();
    fireEvent.pointerDown(divider, { clientX: 400, pointerId: 2 });
    fireEvent.pointerMove(window, { clientX: 460, pointerId: 2 });
    fireEvent.pointerUp(window, { pointerId: 2 });
  });

  it("connects smb share from dialog", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({ smbDialogOpen: true });
    vi.mocked(api.listSmbShares).mockResolvedValue([
      { name: "media", comment: "" },
    ]);
    vi.mocked(api.mountSmbForBrowse).mockResolvedValue("/tmp/media");
    vi.mocked(api.listFolderChildren).mockResolvedValue([
      { name: "Photos", path: "/tmp/media/Photos" },
    ]);
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(
      screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"),
      "10.0.0.1",
    );
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await user.click(await screen.findByText("media"));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await user.click(await screen.findByText("Photos"));
    await user.click(screen.getByRole("button", { name: "Add" }));
    expect(library.actions.connectSmbShare).toHaveBeenCalled();
  });

  it("adds mounted smb path from dialog", async () => {
    const user = userEvent.setup();
    const setSmbDialogOpen = vi.fn();
    const library = createMockLibrary({
      smbDialogOpen: true,
      setSmbDialogOpen,
    });
    pickFolder.mockResolvedValue("/Volumes/nas/photos");
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Choose folder" }));
    expect(pickFolder).toHaveBeenCalled();
    expect(setSmbDialogOpen).toHaveBeenCalledWith(false);
    expect(library.actions.addMountedSmbPath).toHaveBeenCalledWith(
      "/Volumes/nas/photos",
    );
  });

  it("falls back to grid when full view has no selected id", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        fullView: true,
        selectedId: null,
        selectedIds: new Set<number>(),
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getByRole("button", { name: "card" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Next")).not.toBeInTheDocument();
  });

  it("resizes inspector panel in grid view", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        selectedIds: new Set([1]),
        selectedId: 1,
        detail: sampleDetail,
      }),
    );
    const { container } = render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    const dividers = container.querySelectorAll(".cursor-col-resize");
    const divider = dividers[dividers.length - 1] as HTMLElement;
    divider.setPointerCapture = vi.fn();
    divider.releasePointerCapture = vi.fn();
    fireEvent.pointerDown(divider, { clientX: 400, pointerId: 2 });
    fireEvent.pointerMove(window, { clientX: 460, pointerId: 2 });
    fireEvent.pointerUp(window, { pointerId: 2 });
    expect(divider).toBeInTheDocument();
  });

  it("toggles compare tag and album from item bars", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      compareOpen: true,
      compareItems: [sampleCard, sampleVideoCard],
      compareDetails: {
        1: { tag_ids: [1], album_ids: [] },
        2: { tag_ids: [], album_ids: [] },
      },
      albums: [
        {
          id: 1,
          name: "Trip",
          emoji: "📷",
          sort_mode: "date:desc",
          asset_count: 0,
        },
      ],
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getAllByText("Tag")[0]);
    await user.click(screen.getAllByText("trip")[1].closest("button")!);
    expect(library.actions.toggleTagOnAsset).toHaveBeenCalledWith(1, 1, false);
    await user.click(screen.getAllByText("Album")[0]);
    const albumPopover = document.querySelector(
      ".surface-popover",
    ) as HTMLElement;
    await user.click(within(albumPopover).getByText("Trip"));
    expect(library.actions.toggleAlbumOnAsset).toHaveBeenCalledWith(1, 1, true);
  });

  it("creates compare tag and album from item bars", async () => {
    const user = userEvent.setup();
    const library = createMockLibrary({
      compareOpen: true,
      compareItems: [sampleCard, sampleVideoCard],
      compareDetails: {
        1: { tag_ids: [], album_ids: [] },
        2: { tag_ids: [], album_ids: [] },
      },
      tags: [],
      albums: [],
    });
    useLibraryMock.mockReturnValue(library);
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    await user.click(screen.getAllByText("Tag")[0]);
    await user.type(
      screen.getAllByPlaceholderText("New tag")[0],
      "beach{Enter}",
    );
    expect(library.actions.createTagOnAsset).toHaveBeenCalledWith(1, "beach");
    await user.click(screen.getAllByText("Album")[0]);
    await user.type(
      screen.getAllByPlaceholderText("New album")[0],
      "Summer{Enter}",
    );
    expect(library.actions.createAlbumOnAsset).toHaveBeenCalledWith(
      1,
      "Summer",
    );
  });

  it("falls back to grid when full view selected id is missing from items", () => {
    useLibraryMock.mockReturnValue(
      createMockLibrary({
        fullView: true,
        selectedId: 999,
        selectedIds: new Set([999]),
        items: [sampleCard],
      }),
    );
    render(<LibraryApp workspaceId="ws-test" onCloseWorkspace={vi.fn()} />);
    expect(screen.getByRole("button", { name: "card" })).toBeInTheDocument();
  });
});
