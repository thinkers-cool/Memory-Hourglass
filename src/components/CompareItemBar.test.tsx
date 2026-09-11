import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CompareItemBar } from "./CompareItemBar";

const tags = [
  { id: 1, name: "travel", color: "#ff0000", parent_id: null, asset_count: 0 },
];
const albums = [
  { id: 1, name: "Trip", emoji: "📷", sort_mode: "date:desc", asset_count: 0 },
];

function renderBar(overrides: Partial<Parameters<typeof CompareItemBar>[0]> = {}) {
  const props = {
    rating: 3,
    tags,
    albums,
    busy: false,
    tagMenuOpen: false,
    onTagMenuOpenChange: vi.fn(),
    albumMenuOpen: false,
    onAlbumMenuOpenChange: vi.fn(),
    onRate: vi.fn(),
    onToggleTag: vi.fn(),
    onCreateTag: vi.fn(),
    onToggleAlbum: vi.fn(),
    onCreateAlbum: vi.fn(),
    onExport: vi.fn(),
    onDelete: vi.fn(),
    selectedTagKeys: [],
    selectedAlbumKeys: [],
    ...overrides,
  };
  render(<CompareItemBar {...props} />);
  return props;
}

describe("CompareItemBar", () => {
  it("renders item action toolbar", () => {
    renderBar();
    expect(screen.getByRole("toolbar", { name: "Item actions" })).toBeInTheDocument();
    expect(screen.getByText("Tag")).toBeInTheDocument();
    expect(screen.getByText("Album")).toBeInTheDocument();
  });

  it("calls export and delete handlers", async () => {
    const user = userEvent.setup();
    const props = renderBar();
    await user.click(screen.getByRole("button", { name: "Export" }));
    await user.click(screen.getByRole("button", { name: "Delete" }));
    expect(props.onExport).toHaveBeenCalledTimes(1);
    expect(props.onDelete).toHaveBeenCalledTimes(1);
  });

  it("calls onRate when a star is clicked", async () => {
    const user = userEvent.setup();
    const props = renderBar();
    await user.click(screen.getByRole("button", { name: "Rate 5 stars" }));
    expect(props.onRate).toHaveBeenCalledWith(5);
  });

  it("opens tag and album popovers", async () => {
    const user = userEvent.setup();
    const onTagMenuOpenChange = vi.fn();
    const onAlbumMenuOpenChange = vi.fn();
    renderBar({ onTagMenuOpenChange, onAlbumMenuOpenChange });
    await user.click(screen.getByText("Tag"));
    expect(onTagMenuOpenChange).toHaveBeenCalledWith(true);
    await user.click(screen.getByText("Album"));
    expect(onAlbumMenuOpenChange).toHaveBeenCalledWith(true);
  });

  it("toggles tag selection when popover is open", async () => {
    const user = userEvent.setup();
    const props = renderBar({ tagMenuOpen: true });
    await user.click(screen.getByText("travel"));
    expect(props.onToggleTag).toHaveBeenCalledWith(1, true);
  });
});
