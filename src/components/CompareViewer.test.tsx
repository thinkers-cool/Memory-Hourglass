import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CompareViewer } from "./CompareViewer";
import type { AssetCard } from "../types";

const cardA: AssetCard = {
  id: 1,
  file_name: "photo-a.jpg",
  ext: "jpg",
  kind: "image",
  capture_at: null,
  rating: 3,
  sync_state: "ok",
  thumb_path: null,
  abs_path: "/tmp/1.jpg",
  has_duplicate: false,
};

const cardB: AssetCard = {
  ...cardA,
  id: 2,
  file_name: "photo-b.jpg",
  abs_path: "/tmp/2.mp4",
  kind: "video",
  ext: "mp4",
};

function renderViewer(overrides: Partial<Parameters<typeof CompareViewer>[0]> = {}) {
  const props = {
    items: [cardA, cardB],
    tags: [{ id: 1, name: "trip", color: null, parent_id: null, asset_count: 0 }],
    albums: [{ id: 2, name: "Set", emoji: null, sort_mode: "date:desc", asset_count: 0 }],
    busy: false,
    compareDetails: {
      1: { tag_ids: [1], album_ids: [2] },
      2: { tag_ids: [], album_ids: [] },
    },
    stampArmed: false,
    onToggleStamp: vi.fn(),
    onClose: vi.fn(),
    onRate: vi.fn(),
    onToggleTag: vi.fn(),
    onCreateTag: vi.fn(),
    onToggleAlbum: vi.fn(),
    onCreateAlbum: vi.fn(),
    onExport: vi.fn(),
    onDelete: vi.fn(),
    ...overrides,
  };
  render(<CompareViewer {...props} />);
  return props;
}

describe("CompareViewer", () => {
  it("returns null with fewer than two items", () => {
    const { container } = render(
      <CompareViewer
        items={[cardA]}
        tags={[]}
        albums={[]}
        busy={false}
        compareDetails={{}}
        stampArmed={false}
        onToggleStamp={vi.fn()}
        onClose={vi.fn()}
        onRate={vi.fn()}
        onToggleTag={vi.fn()}
        onCreateTag={vi.fn()}
        onToggleAlbum={vi.fn()}
        onCreateAlbum={vi.fn()}
        onExport={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders compare items and closes on escape", async () => {
    const user = userEvent.setup();
    const props = renderViewer();
    expect(screen.getByText("1 / 2 — photo-a.jpg")).toBeInTheDocument();
    expect(screen.getByText("2 / 2 — photo-b.jpg")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(props.onClose).toHaveBeenCalledTimes(1);
  });

  it("stamps on space when armed", () => {
    const props = renderViewer({ stampArmed: true });
    window.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }));
    expect(props.onToggleStamp).toHaveBeenCalledTimes(1);
  });

  it("renders video pane and toolbar actions", async () => {
    const user = userEvent.setup();
    const props = renderViewer();
    expect(document.querySelector("video")).toBeInTheDocument();
    await user.click(screen.getAllByRole("button", { name: "Export" })[0]);
    await user.click(screen.getAllByRole("button", { name: "Delete" })[0]);
    expect(props.onExport).toHaveBeenCalledWith(1);
    expect(props.onDelete).toHaveBeenCalledWith(1);
  });

  it("closes from pane close button on last column", async () => {
    const user = userEvent.setup();
    const props = renderViewer();
    await user.click(screen.getByLabelText("Close"));
    expect(props.onClose).toHaveBeenCalled();
  });

  it("routes rating and tag menu changes", async () => {
    const user = userEvent.setup();
    const props = renderViewer();
    await user.click(screen.getAllByRole("button", { name: /Rate 4 stars/i })[0]);
    expect(props.onRate).toHaveBeenCalledWith(1, 4);
    await user.click(screen.getAllByText("Tag")[0]);
    expect(screen.getAllByText("Tag").length).toBeGreaterThan(0);
  });

  it("opens album menu on second pane and toggles album", async () => {
    const user = userEvent.setup();
    const props = renderViewer();
    await user.click(screen.getAllByText("Album")[1]);
    await user.click(screen.getByText("Set"));
    expect(props.onToggleAlbum).toHaveBeenCalledWith(2, 2, true);
  });

  it("renders image pane and zoom controls", () => {
    renderViewer();
    expect(document.querySelector("img")).toBeInTheDocument();
    expect(screen.getAllByLabelText("Zoom in").length).toBeGreaterThan(0);
    expect(screen.getAllByLabelText("Zoom out").length).toBeGreaterThan(0);
  });

  it("ignores space when stamp is not armed", () => {
    const props = renderViewer({ stampArmed: false });
    window.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }));
    expect(props.onToggleStamp).not.toHaveBeenCalled();
  });

  it("closes only from last pane close button", async () => {
    const user = userEvent.setup();
    const props = renderViewer();
    const closeButtons = screen.getAllByLabelText("Close");
    expect(closeButtons).toHaveLength(1);
    await user.click(closeButtons[0]);
    expect(props.onClose).toHaveBeenCalledTimes(1);
  });

  it("uses zoom controls on image pane", async () => {
    const user = userEvent.setup();
    renderViewer();
    await user.click(screen.getAllByLabelText("Zoom in")[0]);
    await user.click(screen.getAllByLabelText("Zoom out")[0]);
    await user.click(screen.getAllByLabelText("Fit to view")[0]);
    expect(screen.getAllByLabelText("Zoom in")[0]).toBeInTheDocument();
  });

  it("handles missing compare details for items", () => {
    renderViewer({ compareDetails: {} });
    expect(screen.getByText("1 / 2 — photo-a.jpg")).toBeInTheDocument();
  });

  it("closes album menu when tag menu opens", async () => {
    const user = userEvent.setup();
    renderViewer();
    await user.click(screen.getAllByText("Album")[1]);
    expect(screen.getByText("Set")).toBeInTheDocument();
    await user.click(screen.getAllByText("Tag")[0]);
    expect(screen.queryByText("Set")).not.toBeInTheDocument();
  });

  it("closes tag menu when album menu opens", async () => {
    const user = userEvent.setup();
    renderViewer();
    await user.click(screen.getAllByText("Tag")[0]);
    expect(screen.getByText("trip")).toBeInTheDocument();
    await user.click(screen.getAllByText("Album")[1]);
    expect(screen.queryByText("trip")).not.toBeInTheDocument();
  });

  it("toggles tag off when already selected", async () => {
    const user = userEvent.setup();
    const props = renderViewer();
    await user.click(screen.getAllByText("Tag")[0]);
    await user.click(screen.getByText("trip"));
    expect(props.onToggleTag).toHaveBeenCalledWith(1, 1, false);
  });

  it("creates album from compare item bar", async () => {
    const user = userEvent.setup();
    const props = renderViewer();
    await user.click(screen.getAllByText("Album")[0]);
    const input = screen.getAllByPlaceholderText("New album")[0];
    await user.type(input!, "Beach{Enter}");
    expect(props.onCreateAlbum).toHaveBeenCalledWith(1, "Beach");
  });

  it("creates tag from compare item bar", async () => {
    const user = userEvent.setup();
    const props = renderViewer();
    await user.click(screen.getAllByText("Tag")[0]);
    const input = screen.getAllByPlaceholderText("New tag")[0];
    await user.type(input!, "beach{Enter}");
    expect(props.onCreateTag).toHaveBeenCalledWith(1, "beach");
  });
});
