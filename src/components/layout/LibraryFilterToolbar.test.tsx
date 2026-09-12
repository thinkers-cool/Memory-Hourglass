import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { emptyFilterBar } from "../../test/fixtures";
import { EMPTY_STAMP_CONFIG } from "../../lib/stamp";
import { LibraryFilterToolbar } from "./LibraryFilterToolbar";

const tags = [{ id: 1, name: "trip", parent_id: null, color: null }];
const albums = [
  {
    id: 1,
    name: "Vacation",
    emoji: "📷",
    sort_mode: "date:desc",
    asset_count: 0,
  },
];

function renderToolbar(
  overrides: Partial<Parameters<typeof LibraryFilterToolbar>[0]> = {},
) {
  const props = {
    tags,
    albums,
    filterBar: emptyFilterBar,
    setFilterBar: vi.fn(),
    gridColumnCount: 5,
    onGridColumnCountChange: vi.fn(),
    onAdjustGridSize: vi.fn(),
    onSaveCollection: vi.fn(),
    onOpenSlideshow: vi.fn(),
    onOpenExport: vi.fn(),
    slideshowDisabled: false,
    stampConfig: EMPTY_STAMP_CONFIG,
    stampArmed: false,
    onDisarmStamp: vi.fn(),
    onStampRatingChange: vi.fn(),
    onToggleStampTag: vi.fn(),
    onToggleStampAlbum: vi.fn(),
    ...overrides,
  };
  render(<LibraryFilterToolbar {...props} />);
  return props;
}

describe("LibraryFilterToolbar", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("opens slideshow and export actions", async () => {
    const user = userEvent.setup();
    const onOpenSlideshow = vi.fn();
    const onOpenExport = vi.fn();
    render(
      <LibraryFilterToolbar
        tags={[]}
        albums={[]}
        filterBar={emptyFilterBar}
        setFilterBar={vi.fn()}
        gridColumnCount={5}
        onGridColumnCountChange={vi.fn()}
        onAdjustGridSize={vi.fn()}
        onSaveCollection={vi.fn()}
        onOpenSlideshow={onOpenSlideshow}
        onOpenExport={onOpenExport}
        slideshowDisabled={false}
        stampConfig={EMPTY_STAMP_CONFIG}
        stampArmed={false}
        onDisarmStamp={vi.fn()}
        onStampRatingChange={vi.fn()}
        onToggleStampTag={vi.fn()}
        onToggleStampAlbum={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Slideshow" }));
    await user.click(screen.getByRole("button", { name: "Export" }));
    expect(onOpenSlideshow).toHaveBeenCalledTimes(1);
    expect(onOpenExport).toHaveBeenCalledTimes(1);
  });

  it("hides action labels when toolbar is narrow", async () => {
    const rect = {
      width: 640,
      height: 40,
      top: 0,
      left: 0,
      right: 640,
      bottom: 40,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    };
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(
      rect as DOMRect,
    );

    render(
      <LibraryFilterToolbar
        tags={[]}
        albums={[]}
        filterBar={emptyFilterBar}
        setFilterBar={vi.fn()}
        gridColumnCount={5}
        onGridColumnCountChange={vi.fn()}
        onAdjustGridSize={vi.fn()}
        onSaveCollection={vi.fn()}
        onOpenSlideshow={vi.fn()}
        onOpenExport={vi.fn()}
        slideshowDisabled={false}
        stampConfig={EMPTY_STAMP_CONFIG}
        stampArmed={false}
        onDisarmStamp={vi.fn()}
        onStampRatingChange={vi.fn()}
        onToggleStampTag={vi.fn()}
        onToggleStampAlbum={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "Export" })).toBeInTheDocument();
    expect(screen.queryByText("Export")).not.toBeInTheDocument();
    expect(screen.queryByText("Slideshow")).not.toBeInTheDocument();
    expect(screen.queryByText("Save as Collection")).not.toBeInTheDocument();
  });

  it("changes sort via sort dropdown", async () => {
    const user = userEvent.setup();
    const { setFilterBar } = renderToolbar();
    await user.click(screen.getByRole("button", { name: /Date/i }));
    await user.click(screen.getByRole("button", { name: "Name" }));
    expect(setFilterBar).toHaveBeenCalled();
    const updated = setFilterBar.mock.calls
      .map(([arg]) => (typeof arg === "function" ? arg(emptyFilterBar) : arg))
      .find((bar) => bar.sort === "name");
    expect(updated?.sort).toBe("name");
  });

  it("adds camera filter via chip bar", async () => {
    const user = userEvent.setup();
    const { setFilterBar } = renderToolbar();
    await user.click(screen.getByRole("button", { name: /Add filter/i }));
    await user.click(screen.getByRole("button", { name: "Camera" }));
    const input = await screen.findByPlaceholderText("Filter by Camera");
    fireEvent.change(input, { target: { value: "Sony" } });
    const updated = setFilterBar.mock.calls
      .map(([arg]) => (typeof arg === "function" ? arg(emptyFilterBar) : arg))
      .find((bar) => bar.camera === "Sony");
    expect(updated?.camera).toBe("Sony");
  });

  it("removes camera filter via chip bar", async () => {
    const user = userEvent.setup();
    const { setFilterBar } = renderToolbar({
      filterBar: { ...emptyFilterBar, camera: "Sony" },
    });
    await user.click(screen.getByRole("button", { name: "Remove Camera" }));
    expect(setFilterBar).toHaveBeenCalled();
  });

  it("clears active filters", async () => {
    const user = userEvent.setup();
    const { setFilterBar } = renderToolbar({
      filterBar: { ...emptyFilterBar, camera: "Sony" },
    });
    await user.click(screen.getByRole("button", { name: /Clear all/i }));
    expect(setFilterBar).toHaveBeenCalledWith(emptyFilterBar);
  });

  it("updates capture date range via chip bar", async () => {
    const user = userEvent.setup();
    const { setFilterBar } = renderToolbar({
      filterBar: {
        ...emptyFilterBar,
        captureFrom: "2024-01-01",
        captureTo: "",
      },
    });
    await user.click(screen.getByRole("button", { name: /from 2024-01-01/i }));
    await user.type(screen.getByLabelText("To"), "2024-12-31");
    expect(setFilterBar).toHaveBeenCalled();
    const updater = setFilterBar.mock.calls.at(-1)?.[0] as (
      prev: typeof emptyFilterBar,
    ) => typeof emptyFilterBar;
    expect(updater(emptyFilterBar).captureTo).toBe("2024-12-31");
  });
});
