import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { DEFAULT_ALBUM_EMOJI, DEFAULT_TAG_COLOR } from "../../lib/libraryIndicators";
import {
  emptyFilterBar,
  mockLibraryActions,
} from "../../test/fixtures";
import type { RootStats } from "../../types";
import { LibraryPanel } from "./LibraryPanel";

const sampleRoot: RootStats = {
  id: 5,
  path: "/mnt/nas/kenney_holiday-kit",
  kind: "local",
  status: "ok",
  scan_policy: "watch",
  poll_secs: null,
  last_scan_at: null,
  asset_count: 108,
  missing_count: 0,
};

function renderPanel(
  overrides: Partial<Parameters<typeof LibraryPanel>[0]> = {},
) {
  const actions = overrides.actions ?? mockLibraryActions();
  const props = {
    tab: "library" as const,
    roots: [] as RootStats[],
    albums: [],
    collections: [],
    tags: [],
    filterBar: emptyFilterBar,
    extraFilter: {},
    selectedCollectionId: null,
    deleteStatus: "",
    deletedCount: 0,
    busy: false,
    actions,
    ...overrides,
  };
  return { actions, ...render(<LibraryPanel {...props} />) };
}

describe("LibraryPanel", () => {
  it("renders library source rows", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <LibraryPanel
        tab="library"
        roots={[
          {
            id: 1,
            path: "/mnt/nas/kenney_holiday-kit",
            kind: "local",
            status: "ok",
            scan_policy: "watch",
            poll_secs: null,
            last_scan_at: null,
            asset_count: 108,
            missing_count: 0,
          },
        ]}
        albums={[]}
        collections={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );
    expect(screen.getByText("kenney_holiday-kit")).toBeInTheDocument();
    expect(screen.getByText("108 items")).toBeInTheDocument();
    await user.click(screen.getByText("kenney_holiday-kit"));
    expect(actions.selectRoot).toHaveBeenCalledWith(1);
  });

  it("renders library sources panel", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <LibraryPanel
        tab="library"
        roots={[]}
        albums={[]}
        collections={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );
    expect(screen.getByText("No sources yet")).toBeInTheDocument();
    await user.click(screen.getByTitle("Add folder"));
    expect(actions.addLocalRoot).toHaveBeenCalledTimes(1);
  });

  it("renders collections tab", () => {
    render(
      <LibraryPanel
        tab="collections"
        roots={[]}
        albums={[]}
        collections={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={mockLibraryActions()}
      />,
    );
    expect(screen.getByText("Save filters to create one")).toBeInTheDocument();
  });

  it("deletes tags from the library panel", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <LibraryPanel
        tab="library"
        roots={[]}
        albums={[]}
        collections={[]}
        tags={[
          {
            id: 7,
            name: "travel",
            parent_id: null,
            color: "#ff0000",
            asset_count: 3,
          },
        ]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );

    await user.click(screen.getAllByTitle("Remove")[0]);
    expect(actions.deleteTag).toHaveBeenCalledWith(7);
  });

  it("adds subtags from the tag row action", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <LibraryPanel
        tab="library"
        roots={[]}
        albums={[]}
        collections={[]}
        tags={[
          {
            id: 7,
            name: "travel",
            parent_id: null,
            color: "#ff0000",
            asset_count: 3,
          },
        ]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );

    await user.click(screen.getByTitle("Add subtag"));
    await user.type(screen.getByPlaceholderText("New subtag"), "japan");
    await user.click(screen.getByTitle("Save"));
    expect(actions.createTag).toHaveBeenCalledWith("japan", 7, "#6b7280");
  });

  it("opens smb connect from library panel", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <LibraryPanel
        tab="library"
        roots={[]}
        albums={[]}
        collections={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );
    await user.click(screen.getByTitle("Connect SMB share"));
    expect(actions.openSmbConnect).toHaveBeenCalledTimes(1);
  });

  it("selects albums and deletes them", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <LibraryPanel
        tab="library"
        roots={[]}
        albums={[
          {
            id: 3,
            name: "Summer",
            emoji: "☀️",
            sort_mode: "date:desc",
            asset_count: 4,
            created_at: 0,
          },
        ]}
        collections={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );
    await user.click(screen.getByText("Summer"));
    expect(actions.selectAlbum).toHaveBeenCalledWith(3);
    await user.click(screen.getAllByTitle("Remove")[0]);
    expect(actions.deleteAlbum).toHaveBeenCalledWith(3);
  });

  it("creates a new album from the panel", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <LibraryPanel
        tab="library"
        roots={[]}
        albums={[]}
        collections={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );
    await user.click(screen.getByTitle("New album"));
    await user.type(screen.getByPlaceholderText("New album"), "Trips");
    await user.click(screen.getByTitle("Save"));
    expect(actions.createAlbum).toHaveBeenCalledWith("Trips", DEFAULT_ALBUM_EMOJI);
  });

  it("switches to trash and back", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    const { rerender } = render(
      <LibraryPanel
        tab="library"
        roots={[]}
        albums={[]}
        collections={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={2}
        busy={false}
        actions={actions}
      />,
    );
    await user.click(screen.getByText("Deleted"));
    expect(actions.viewTrash).toHaveBeenCalledTimes(1);

    rerender(
      <LibraryPanel
        tab="library"
        roots={[]}
        albums={[]}
        collections={[]}
        tags={[]}
        filterBar={{ ...emptyFilterBar, deleteStatus: "deleted" }}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus="deleted"
        deletedCount={2}
        busy={false}
        actions={actions}
      />,
    );
    await user.click(screen.getByText("Deleted"));
    expect(actions.viewLibrary).toHaveBeenCalledTimes(1);
  });

  it("selects and deletes collections", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <LibraryPanel
        tab="collections"
        roots={[]}
        albums={[]}
        collections={[
          {
            id: 9,
            name: "Favorites",
            filter_json: "{}",
            asset_count: 12,
            created_at: 0,
          },
        ]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        selectedCollectionId={null}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );
    await user.click(screen.getByText("Favorites"));
    expect(actions.selectCollection).toHaveBeenCalled();
    await user.click(screen.getByTitle("Remove"));
    expect(actions.deleteCollection).toHaveBeenCalledWith(9);
  });

  it("runs source row sync relink and remove actions", async () => {
    const user = userEvent.setup();
    const { actions } = renderPanel({ roots: [sampleRoot] });
    await user.click(screen.getByTitle("Sync"));
    expect(actions.syncRoot).toHaveBeenCalledWith(5);
    await user.click(screen.getByTitle("Relink"));
    expect(actions.relinkRoot).toHaveBeenCalledWith(5);
    await user.click(screen.getAllByTitle("Remove")[0]);
    expect(actions.removeRoot).toHaveBeenCalledWith(5);
    expect(actions.selectRoot).not.toHaveBeenCalled();
  });

  it("renders offline smb and flat-path sources", () => {
    renderPanel({
      roots: [
        { ...sampleRoot, id: 1, path: "photos", status: "offline" },
        {
          ...sampleRoot,
          id: 2,
          path: "//nas/media",
          kind: "smb",
          status: "ok",
        },
        {
          ...sampleRoot,
          id: 3,
          path: "/mnt/nas/",
          status: "ok",
        },
      ],
    });
    expect(screen.getByText("photos")).toBeInTheDocument();
    expect(screen.getByText("Offline")).toBeInTheDocument();
    expect(screen.getByTitle("Network source")).toBeInTheDocument();
    expect(screen.getByText("/mnt/nas/")).toBeInTheDocument();
  });

  it("edits albums on double click", async () => {
    const user = userEvent.setup();
    const { actions } = renderPanel({
      albums: [
        {
          id: 3,
          name: "Summer",
          emoji: "☀️",
          sort_mode: "date:desc",
          asset_count: 4,
          created_at: 0,
        },
      ],
    });
    await user.dblClick(screen.getByText("Summer"));
    const input = screen.getByDisplayValue("Summer");
    await user.clear(input);
    await user.type(input, "Winter");
    await user.click(screen.getByTitle("Save"));
    expect(actions.updateAlbum).toHaveBeenCalledWith(3, "Winter", "☀️");
  });

  it("cancels album edit with escape", async () => {
    const user = userEvent.setup();
    const { actions } = renderPanel({
      albums: [
        {
          id: 3,
          name: "Summer",
          emoji: "☀️",
          sort_mode: "date:desc",
          asset_count: 4,
          created_at: 0,
        },
      ],
    });
    await user.dblClick(screen.getByText("Summer"));
    await user.keyboard("{Escape}");
    expect(screen.queryByDisplayValue("Summer")).not.toBeInTheDocument();
    expect(actions.updateAlbum).not.toHaveBeenCalled();
  });

  it("submits album add with enter and cancels with escape", async () => {
    const user = userEvent.setup();
    const { actions } = renderPanel();
    await user.click(screen.getByTitle("New album"));
    const input = screen.getByPlaceholderText("New album");
    await user.type(input, "Trips{Enter}");
    expect(actions.createAlbum).toHaveBeenCalledWith("Trips", DEFAULT_ALBUM_EMOJI);
    await user.click(screen.getByTitle("New album"));
    await user.type(screen.getByPlaceholderText("New album"), "Draft");
    await user.keyboard("{Escape}");
    expect(actions.createAlbum).toHaveBeenCalledTimes(1);
  });

  it("creates root tags and filters by tag click", async () => {
    const user = userEvent.setup();
    const { actions } = renderPanel({
      tags: [
        {
          id: 7,
          name: "travel",
          parent_id: null,
          color: "#ff0000",
          asset_count: 3,
        },
      ],
    });
    await user.click(screen.getByText("travel"));
    expect(actions.filterByTag).toHaveBeenCalledWith(7);
    await user.click(screen.getByTitle("New tag"));
    await user.type(screen.getByPlaceholderText("New tag"), "nature{Enter}");
    expect(actions.createTag).toHaveBeenCalledWith("nature", undefined, DEFAULT_TAG_COLOR);
  });

  it("edits tags on double click", async () => {
    const user = userEvent.setup();
    const { actions } = renderPanel({
      tags: [
        {
          id: 7,
          name: "travel",
          parent_id: null,
          color: "#ff0000",
          asset_count: 3,
        },
      ],
    });
    await user.dblClick(screen.getByText("travel"));
    const input = screen.getByDisplayValue("travel");
    await user.clear(input);
    await user.type(input, "vacation");
    await user.click(screen.getByTitle("Save"));
    expect(actions.updateTag).toHaveBeenCalledWith(7, "vacation", "#ff0000");
  });

  it("renders nested tags and clears subtag add when opening root tag add", async () => {
    const user = userEvent.setup();
    const { actions } = renderPanel({
      tags: [
        {
          id: 1,
          name: "parent",
          parent_id: null,
          color: "#111111",
          asset_count: 1,
        },
        {
          id: 2,
          name: "child",
          parent_id: 1,
          color: "#222222",
          asset_count: 2,
        },
      ],
    });
    expect(screen.getByTitle("Subtag")).toBeInTheDocument();
    await user.click(screen.getAllByTitle("Add subtag")[0]);
    expect(screen.getByPlaceholderText("New subtag")).toBeInTheDocument();
    await user.click(screen.getByTitle("New tag"));
    expect(screen.queryByPlaceholderText("New subtag")).not.toBeInTheDocument();
    expect(screen.getByPlaceholderText("New tag")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(actions.createTag).not.toHaveBeenCalled();
  });

  it("returns null for unknown tab", () => {
    const { container } = renderPanel({ tab: "settings" as never });
    expect(container.firstChild).toBeNull();
  });

  it("ignores empty album and tag commits", async () => {
    const user = userEvent.setup();
    const { actions } = renderPanel({
      albums: [
        {
          id: 3,
          name: "Summer",
          emoji: null,
          sort_mode: "date:desc",
          asset_count: 4,
          created_at: 0,
        },
      ],
      tags: [
        {
          id: 7,
          name: "travel",
          parent_id: null,
          color: null,
          asset_count: 3,
        },
      ],
    });
    await user.click(screen.getByTitle("New album"));
    await user.type(screen.getByPlaceholderText("New album"), "   {Enter}");
    expect(actions.createAlbum).not.toHaveBeenCalled();
    await user.click(screen.getByTitle("New tag"));
    await user.type(screen.getByPlaceholderText("New tag"), "   {Enter}");
    expect(actions.createTag).not.toHaveBeenCalled();
    await user.dblClick(screen.getByText("Summer"));
    const albumInput = screen.getByDisplayValue("Summer");
    await user.clear(albumInput);
    await user.keyboard("{Enter}");
    expect(actions.updateAlbum).not.toHaveBeenCalled();
    await user.dblClick(screen.getByText("travel"));
    const tagInput = screen.getByDisplayValue("travel");
    await user.clear(tagInput);
    await user.keyboard("{Enter}");
    expect(actions.updateTag).not.toHaveBeenCalled();
    await user.keyboard("{Escape}");
    expect(actions.updateTag).not.toHaveBeenCalled();
  });
});
