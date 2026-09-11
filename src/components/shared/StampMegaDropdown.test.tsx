import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { EMPTY_STAMP_CONFIG } from "../../lib/stamp";
import { StampMegaDropdown } from "./StampMegaDropdown";
import type { Album, TagDto } from "../../types";

const tags: TagDto[] = [
  { id: 1, name: "Travel", parent_id: null, color: "#f00", asset_count: 0 },
];

const albums: Album[] = [
  { id: 10, name: "Best", sort_mode: "date:desc", emoji: "⭐", asset_count: 0 },
];

describe("StampMegaDropdown", () => {
  it("updates rating tags and albums", async () => {
    const user = userEvent.setup();
    const onRatingChange = vi.fn();
    const onToggleTag = vi.fn();
    const onToggleAlbum = vi.fn();

    render(
      <StampMegaDropdown
        config={EMPTY_STAMP_CONFIG}
        tags={tags}
        albums={albums}
        onRatingChange={onRatingChange}
        onToggleTag={onToggleTag}
        onToggleAlbum={onToggleAlbum}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Rate 3 stars" }));
    await user.click(screen.getByRole("button", { name: "Travel" }));
    await user.click(screen.getByRole("button", { name: /Best/ }));

    expect(onRatingChange).toHaveBeenCalledWith(3);
    expect(onToggleTag).toHaveBeenCalledWith(1, true);
    expect(onToggleAlbum).toHaveBeenCalledWith(10, true);
  });

  it("clears rating when selecting the active value again", async () => {
    const user = userEvent.setup();
    const onRatingChange = vi.fn();
    render(
      <StampMegaDropdown
        config={{ ...EMPTY_STAMP_CONFIG, rating: 4 }}
        tags={[]}
        albums={[]}
        onRatingChange={onRatingChange}
        onToggleTag={vi.fn()}
        onToggleAlbum={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Rate 4 stars" }));
    expect(onRatingChange).toHaveBeenCalledWith(null);
  });

  it("marks selected tags and albums with visible active styling", () => {
    render(
      <StampMegaDropdown
        config={{ ...EMPTY_STAMP_CONFIG, tag_ids: [1], album_ids: [10] }}
        tags={tags}
        albums={albums}
        onRatingChange={vi.fn()}
        onToggleTag={vi.fn()}
        onToggleAlbum={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "Travel" }).className).toContain(
      "menu-picker-item-active",
    );
    expect(screen.getByRole("button", { name: /Best/ }).className).toContain(
      "menu-picker-item-active",
    );
  });

  it("shows column headers", () => {
    render(
      <StampMegaDropdown
        config={EMPTY_STAMP_CONFIG}
        tags={[]}
        albums={[]}
        onRatingChange={vi.fn()}
        onToggleTag={vi.fn()}
        onToggleAlbum={vi.fn()}
      />,
    );

    expect(screen.getByText("Rating")).toBeInTheDocument();
    expect(screen.getByText("Tags")).toBeInTheDocument();
    expect(screen.getByText("Albums")).toBeInTheDocument();
  });
});
