import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../../api/client";
import { sampleDetail } from "../../test/fixtures";
import { InspectorPanel } from "./InspectorPanel";

vi.mock("../../api/client", () => ({
  queryAssetActivity: vi.fn(),
  undoActivity: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(api.queryAssetActivity).mockResolvedValue([]);
});

describe("InspectorPanel", () => {
  it("shows file keywords that are not in the tag catalog", () => {
    render(
      <InspectorPanel
        detail={{
          ...sampleDetail,
          tag_ids: [],
          meta: {
            asset_id: 1,
            capture_at: null,
            camera: null,
            lens: null,
            rating: null,
            latitude: null,
            longitude: null,
            keywords_json: '["travel", "orphan"]',
          },
        }}
        tags={[
          {
            id: 1,
            name: "travel",
            parent_id: null,
            color: null,
            asset_count: 0,
          },
        ]}
        onClose={vi.fn()}
        onPurge={vi.fn()}
        onSelectLink={vi.fn()}
        onSoftDeleteDuplicate={vi.fn()}
      />,
    );
    expect(screen.getByText("File keywords")).toBeInTheDocument();
    expect(screen.getByText("orphan")).toBeInTheDocument();
    expect(screen.queryByText("travel")).not.toBeInTheDocument();
  });

  it("shows asset metadata and closes", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <InspectorPanel
        detail={sampleDetail}
        tags={[
          {
            id: 1,
            name: "travel",
            parent_id: null,
            color: "#ff0000",
            asset_count: 0,
          },
        ]}
        onClose={onClose}
        onPurge={vi.fn()}
        onSelectLink={vi.fn()}
        onSoftDeleteDuplicate={vi.fn()}
      />,
    );
    expect(screen.getByText("photo.jpg")).toBeInTheDocument();
    expect(screen.getByText("Sony")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "×" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("shows undo for reversible activity entries", async () => {
    const user = userEvent.setup();
    const onUndoActivity = vi.fn().mockResolvedValue(undefined);
    vi.mocked(api.queryAssetActivity).mockResolvedValue([
      {
        id: 7,
        seq: 1,
        occurred_at: 1_700_000_000,
        event_type: "asset.tags_added",
        actor: "user",
        correlation_id: null,
        subject_type: "asset",
        subject_id: 1,
        subject_key: null,
        summary: null,
        payload_json: "{}",
        revert_json: "{}",
        undone_at: null,
        reversible: true,
      },
    ]);

    render(
      <InspectorPanel
        detail={sampleDetail}
        tags={[]}
        onClose={vi.fn()}
        onPurge={vi.fn()}
        onSelectLink={vi.fn()}
        onSoftDeleteDuplicate={vi.fn()}
        onUndoActivity={onUndoActivity}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText("Activity")).toBeInTheDocument();
    });
    await user.click(screen.getByText("Activity"));
    await user.click(await screen.findByRole("button", { name: "Undo" }));
    expect(onUndoActivity).toHaveBeenCalledWith(7);
    await waitFor(() => {
      expect(api.queryAssetActivity.mock.calls.length).toBeGreaterThanOrEqual(
        2,
      );
    });
  });

  it("renders video preview and purge action", async () => {
    const user = userEvent.setup();
    const onPurge = vi.fn();
    render(
      <InspectorPanel
        detail={{
          ...sampleDetail,
          asset: {
            ...sampleDetail.asset,
            kind: "video",
            file_name: "clip.mp4",
          },
          display_path: "/tmp/clip.mp4",
        }}
        tags={[]}
        onClose={vi.fn()}
        onPurge={onPurge}
        onSelectLink={vi.fn()}
        onSoftDeleteDuplicate={vi.fn()}
      />,
    );
    expect(document.querySelector("video")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Purge" }));
    expect(onPurge).toHaveBeenCalledTimes(1);
  });

  it("hides purge action when purge is disabled", () => {
    render(
      <InspectorPanel
        detail={sampleDetail}
        tags={[]}
        onClose={vi.fn()}
        onPurge={vi.fn()}
        purgeEnabled={false}
        onSelectLink={vi.fn()}
        onSoftDeleteDuplicate={vi.fn()}
      />,
    );
    expect(
      screen.queryByRole("button", { name: "Purge" }),
    ).not.toBeInTheDocument();
  });

  it("opens linked files and deletes duplicates", async () => {
    const user = userEvent.setup();
    const onSelectLink = vi.fn();
    const onSoftDeleteDuplicate = vi.fn();
    render(
      <InspectorPanel
        detail={{
          ...sampleDetail,
          links: [
            {
              id: 9,
              file_name: "linked.jpg",
              rel_path: "linked.jpg",
              root_path: "/tmp",
            },
          ],
          duplicates: [
            {
              id: 10,
              file_name: "dup.jpg",
              rel_path: "dup.jpg",
              root_path: "/tmp",
            },
          ],
        }}
        tags={[]}
        onClose={vi.fn()}
        onPurge={vi.fn()}
        onSelectLink={onSelectLink}
        onSoftDeleteDuplicate={onSoftDeleteDuplicate}
      />,
    );

    await user.click(screen.getByText("linked.jpg"));
    expect(onSelectLink).toHaveBeenCalledWith(9);

    await user.click(
      screen.getByRole("button", { name: "Delete duplicate dup.jpg" }),
    );
    expect(onSoftDeleteDuplicate).toHaveBeenCalledWith(10);
  });

  it("shows empty metadata state", () => {
    render(
      <InspectorPanel
        detail={{
          ...sampleDetail,
          raw_tags: [],
          meta: {
            ...sampleDetail.meta,
            camera: null,
            lens: null,
            keywords_json: null,
          },
        }}
        tags={[]}
        onClose={vi.fn()}
        onPurge={vi.fn()}
        onSelectLink={vi.fn()}
        onSoftDeleteDuplicate={vi.fn()}
      />,
    );
    expect(screen.getByText("No metadata")).toBeInTheDocument();
  });

  it("opens duplicate file from link button", async () => {
    const user = userEvent.setup();
    const onSelectLink = vi.fn();
    render(
      <InspectorPanel
        detail={{
          ...sampleDetail,
          links: [],
          duplicates: [
            {
              id: 10,
              file_name: "dup.jpg",
              rel_path: "dup.jpg",
              root_path: "/tmp",
            },
          ],
        }}
        tags={[]}
        onClose={vi.fn()}
        onPurge={vi.fn()}
        onSelectLink={onSelectLink}
        onSoftDeleteDuplicate={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /\/tmp\/dup\.jpg/ }));
    expect(onSelectLink).toHaveBeenCalledWith(10);
  });

  it("filters metadata and handles activity load errors", async () => {
    const user = userEvent.setup();
    vi.mocked(api.queryAssetActivity).mockRejectedValueOnce(
      new Error("activity failed"),
    );
    render(
      <InspectorPanel
        detail={sampleDetail}
        tags={[]}
        onClose={vi.fn()}
        onPurge={vi.fn()}
        onSelectLink={vi.fn()}
        onSoftDeleteDuplicate={vi.fn()}
      />,
    );

    await user.type(screen.getByRole("textbox"), "sony");
    expect(screen.getByText("Sony")).toBeInTheDocument();

    await user.clear(screen.getByRole("textbox"));
    await user.type(screen.getByRole("textbox"), "missing");
    expect(screen.getByText("No matching metadata")).toBeInTheDocument();
  });

  it("handles null activity responses", async () => {
    vi.mocked(api.queryAssetActivity).mockResolvedValue(null as never);
    render(
      <InspectorPanel
        detail={sampleDetail}
        tags={[]}
        onClose={vi.fn()}
        onPurge={vi.fn()}
        onSelectLink={vi.fn()}
        onSoftDeleteDuplicate={vi.fn()}
      />,
    );
    await waitFor(() => {
      expect(api.queryAssetActivity).toHaveBeenCalled();
    });
  });

  it("ignores undo when handler is missing", async () => {
    const user = userEvent.setup();
    vi.mocked(api.queryAssetActivity).mockResolvedValue([
      {
        id: 7,
        seq: 1,
        occurred_at: 1_700_000_000,
        event_type: "asset.tags_added",
        actor: "user",
        correlation_id: null,
        subject_type: "asset",
        subject_id: 1,
        subject_key: null,
        summary: null,
        payload_json: "{}",
        revert_json: "{}",
        undone_at: null,
        reversible: true,
      },
    ]);
    render(
      <InspectorPanel
        detail={sampleDetail}
        tags={[]}
        onClose={vi.fn()}
        onPurge={vi.fn()}
        onSelectLink={vi.fn()}
        onSoftDeleteDuplicate={vi.fn()}
      />,
    );
    await waitFor(() =>
      expect(screen.getByText("Activity")).toBeInTheDocument(),
    );
    await user.click(screen.getByText("Activity"));
    expect(
      screen.queryByRole("button", { name: "Undo" }),
    ).not.toBeInTheDocument();
    expect(api.queryAssetActivity).toHaveBeenCalledTimes(1);
  });
});

describe("InspectorPlaceholder", () => {
  it("closes from placeholder panel", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const { InspectorPlaceholder } = await import("./InspectorPanel");
    render(<InspectorPlaceholder onClose={onClose} />);
    await user.click(screen.getByRole("button", { name: "×" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
