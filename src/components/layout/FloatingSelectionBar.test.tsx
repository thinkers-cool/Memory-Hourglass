import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FloatingSelectionBar } from "./FloatingSelectionBar";

const tags = [
  { id: 1, name: "travel", color: "#ff0000", parent_id: null, asset_count: 0 },
];
const albums = [
  { id: 1, name: "Trip", emoji: "📷", sort_mode: "date:desc", asset_count: 0 },
];

function renderBar(
  overrides: Partial<Parameters<typeof FloatingSelectionBar>[0]> = {},
) {
  const props = {
    mode: "library" as const,
    count: 2,
    tags,
    albums,
    busy: false,
    tagMenuOpen: false,
    onTagMenuOpenChange: vi.fn(),
    albumMenuOpen: false,
    onAlbumMenuOpenChange: vi.fn(),
    onClear: vi.fn(),
    rating: 4,
    onRate: vi.fn(),
    onToggleTag: vi.fn(),
    onCreateTag: vi.fn(),
    onToggleAlbum: vi.fn(),
    onCreateAlbum: vi.fn(),
    onExport: vi.fn(),
    onCompare: vi.fn(),
    onDelete: vi.fn(),
    onRestore: vi.fn(),
    onPurge: vi.fn(),
    selectedTagKeys: [],
    selectedAlbumKeys: [],
    ...overrides,
  };
  render(<FloatingSelectionBar {...props} />);
  return props;
}

describe("FloatingSelectionBar", () => {
  it("shows selection count and compare for multi-select", async () => {
    const user = userEvent.setup();
    const props = renderBar();
    expect(screen.getByText("2 selected")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Rate 4 stars" }),
    ).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByRole("button", { name: /Compare/i }));
    expect(props.onCompare).toHaveBeenCalledTimes(1);
  });

  it("hides compare when only one item is selected", () => {
    renderBar({ count: 1 });
    expect(
      screen.queryByRole("button", { name: /Compare/i }),
    ).not.toBeInTheDocument();
  });

  it("calls export, delete, and clear in library mode", async () => {
    const user = userEvent.setup();
    const props = renderBar();
    await user.click(screen.getByRole("button", { name: /Export/i }));
    await user.click(screen.getByRole("button", { name: /Delete/i }));
    await user.click(screen.getByRole("button", { name: "Clear selection" }));
    expect(props.onExport).toHaveBeenCalledTimes(1);
    expect(props.onDelete).toHaveBeenCalledTimes(1);
    expect(props.onClear).toHaveBeenCalledTimes(1);
  });

  it("shows restore and purge in trash mode", async () => {
    const user = userEvent.setup();
    const props = renderBar({ mode: "trash" });
    await user.click(screen.getByRole("button", { name: /Restore/i }));
    await user.click(screen.getByRole("button", { name: /Purge/i }));
    expect(props.onRestore).toHaveBeenCalledTimes(1);
    expect(props.onPurge).toHaveBeenCalledTimes(1);
    expect(
      screen.queryByRole("button", { name: /Delete/i }),
    ).not.toBeInTheDocument();
  });

  it("hides purge in trash mode when purge is disabled", () => {
    renderBar({ mode: "trash", purgeEnabled: false });
    expect(
      screen.getByRole("button", { name: /Restore/i }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Purge/i }),
    ).not.toBeInTheDocument();
  });

  it("disables trash actions when busy", () => {
    renderBar({ mode: "trash", busy: true });
    expect(screen.getByRole("button", { name: /Restore/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Purge/i })).toBeDisabled();
  });

  it("calls onRate and opens tag menu", async () => {
    const user = userEvent.setup();
    const props = renderBar({ tagMenuOpen: true });
    await user.click(screen.getByRole("button", { name: "Rate 2 stars" }));
    expect(props.onRate).toHaveBeenCalledWith(2);
    await user.click(screen.getByText("travel"));
    expect(props.onToggleTag).toHaveBeenCalledWith(1, true);
  });

  it("opens album menu and creates album", async () => {
    const user = userEvent.setup();
    const props = renderBar({ albumMenuOpen: true });
    await user.click(screen.getByText("Trip"));
    expect(props.onToggleAlbum).toHaveBeenCalledWith(1, true);
  });
});
