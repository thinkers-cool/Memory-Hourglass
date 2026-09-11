import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AssetCard } from "../types";

const {
  listRootStats,
  listAlbums,
  listSmartCollections,
  listTags,
  queryAssets,
  countAssets,
  onScanProgress,
  onJobProgress,
  onMessageNotify,
  addRoot,
  startScan,
  pickFolder,
  undoActivity,
} = vi.hoisted(() => ({
  listRootStats: vi.fn(),
  listAlbums: vi.fn(),
  listSmartCollections: vi.fn(),
  listTags: vi.fn(),
  queryAssets: vi.fn(),
  countAssets: vi.fn(),
  onScanProgress: vi.fn(),
  onJobProgress: vi.fn(),
  onMessageNotify: vi.fn(),
  addRoot: vi.fn(),
  startScan: vi.fn(),
  pickFolder: vi.fn(),
  undoActivity: vi.fn(),
}));

vi.mock("../api/client", () => ({
  listRootStats,
  listAlbums,
  listSmartCollections,
  listTags,
  queryAssets,
  countAssets,
  onScanProgress,
  onJobProgress,
  onMessageNotify,
  getAsset: vi.fn().mockResolvedValue({
    asset: { id: 1, file_name: "a.jpg" },
    abs_path: "/tmp/a.jpg",
    display_path: "/tmp/a.jpg",
    tag_ids: [],
    album_ids: [],
    raw_tags: [],
    links: [],
    duplicates: [],
  }),
  addRoot,
  startScan,
  startExport: vi.fn(),
  undoActivity,
}));

vi.mock("../lib/pickFolder", () => ({
  pickFolder,
  formatError: (error: unknown) =>
    error instanceof Error ? error.message : String(error),
}));

import { useLibrary } from "./useLibrary";

const sampleCard: AssetCard = {
  id: 1,
  file_name: "a.jpg",
  ext: "jpg",
  kind: "image",
  capture_at: null,
  rating: null,
  sync_state: "ok",
  thumb_path: null,
  abs_path: "/tmp/a.jpg",
  has_duplicate: false,
};

describe("useLibrary", () => {
  beforeEach(() => {
    countAssets.mockResolvedValue(0);
  });

  it("loads library metadata and grid on mount", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 1, items: [sampleCard] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);

    const { result } = renderHook(() => useLibrary());

    await waitFor(() => expect(result.current.items).toHaveLength(1));
    expect(result.current.total).toBe(1);
    expect(listRootStats).toHaveBeenCalled();
    expect(queryAssets).toHaveBeenCalled();
  });

  it("scopes grid to a selected root", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 0, items: [] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(queryAssets).toHaveBeenCalled());

    act(() => {
      result.current.actions.selectRoot(5);
    });

    await waitFor(() =>
      expect(result.current.extraFilter).toEqual({ root_id: 5 }),
    );
  });

  it("clears selection through actions", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 1, items: [sampleCard] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    await act(async () => {
      await result.current.actions.selectAsset(sampleCard, false, false);
    });
    expect(result.current.selectedIds.has(1)).toBe(true);

    act(() => {
      result.current.actions.clearSelection();
    });
    expect(result.current.selectedIds.size).toBe(0);
  });

  it("closeDetail clears multi-select state", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 1, items: [sampleCard] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    await act(async () => {
      await result.current.actions.selectAsset(sampleCard, false, false);
    });
    expect(result.current.selectedIds.has(1)).toBe(true);

    act(() => {
      result.current.actions.closeDetail();
    });
    expect(result.current.selectedIds.size).toBe(0);
    expect(result.current.selectedId).toBeNull();
    expect(result.current.detail).toBeNull();
  });

  it("closeInspector hides inspector in full view without exiting", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 1, items: [sampleCard] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    await act(async () => {
      await result.current.actions.openFullViewFor(sampleCard);
    });
    expect(result.current.fullView).toBe(true);
    expect(result.current.inspectorVisible).toBe(true);

    act(() => {
      result.current.actions.closeInspector();
    });
    expect(result.current.fullView).toBe(true);
    expect(result.current.inspectorVisible).toBe(false);
    expect(result.current.selectedIds.has(1)).toBe(true);
  });

  it("refreshes root stats when scan completes", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 0, items: [] });
    let scanHandler:
      | ((progress: {
          root_id: number;
          stage: string;
          scanned: number;
          indexed: number;
        }) => void)
      | undefined;
    onMessageNotify.mockResolvedValue(() => undefined);
    onScanProgress.mockImplementation(async (handler) => {
      scanHandler = handler;
      return () => undefined;
    });
    onJobProgress.mockResolvedValue(() => undefined);

    renderHook(() => useLibrary());
    await waitFor(() => expect(onScanProgress).toHaveBeenCalled());

    listRootStats.mockResolvedValue([
      {
        id: 1,
        path: "/tmp/photos",
        kind: "local",
        status: "ok",
        scan_policy: "watch",
        poll_secs: null,
        last_scan_at: null,
        asset_count: 12,
        missing_count: 0,
      },
    ]);

    act(() => {
      scanHandler?.({
        root_id: 1,
        stage: "done",
        scanned: 12,
        indexed: 12,
      });
    });

    await waitFor(() =>
      expect(listRootStats.mock.calls.length).toBeGreaterThanOrEqual(2),
    );
  });

  it("adjusts grid column count", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 0, items: [] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(queryAssets).toHaveBeenCalled());

    const initial = result.current.gridColumnCount;
    act(() => {
      result.current.actions.adjustGridSize(1);
    });
    expect(result.current.gridColumnCount).toBe(initial + 1);
  });

  it("clears selection when selected asset disappears from grid", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 1, items: [sampleCard] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    await act(async () => {
      await result.current.actions.selectAsset(sampleCard, false, false);
    });
    expect(result.current.selectedIds.has(1)).toBe(true);

    queryAssets.mockResolvedValue({ total: 0, items: [] });
    await act(async () => {
      await result.current.actions.clearFilters();
    });

    await waitFor(() => {
      expect(result.current.selectedIds.size).toBe(0);
      expect(result.current.selectedId).toBeNull();
    });
  });

  it("reports folder picker errors", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 0, items: [] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);
    pickFolder.mockRejectedValue(new Error("picker denied"));

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(queryAssets).toHaveBeenCalled());

    await act(async () => {
      await result.current.actions.addLocalRoot();
    });

    expect(addRoot).not.toHaveBeenCalled();
    expect(result.current.notification?.text).toContain("picker denied");
  });

  it("reports scan start failures", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 0, items: [] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);
    pickFolder.mockResolvedValue("/tmp/photos");
    addRoot.mockResolvedValue({ id: 9, path: "/tmp/photos" });
    startScan.mockRejectedValue(new Error("scan failed"));

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(queryAssets).toHaveBeenCalled());

    await act(async () => {
      await result.current.actions.addLocalRoot();
    });

    await waitFor(() => {
      expect(result.current.notification?.text).toContain("scan failed");
    });
  });

  it("undoes activity and refreshes library", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 0, items: [] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);
    undoActivity.mockResolvedValue(undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(queryAssets).toHaveBeenCalled());

    await act(async () => {
      await result.current.undoActivity(42);
    });

    expect(undoActivity).toHaveBeenCalledWith(42);
    expect(listRootStats.mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  it("closes collection dialog", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 0, items: [] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(queryAssets).toHaveBeenCalled());

    act(() => {
      result.current.actions.saveCollection();
    });
    expect(result.current.collectionDialogOpen).toBe(true);

    act(() => {
      result.current.closeCollectionDialog();
    });
    expect(result.current.collectionDialogOpen).toBe(false);
  });

  it("ignores cancelled folder pick", async () => {
    pickFolder.mockReset();
    addRoot.mockReset();
    startScan.mockReset();
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 0, items: [] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);
    pickFolder.mockResolvedValue(null);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(queryAssets).toHaveBeenCalled());

    await act(async () => {
      await result.current.actions.addLocalRoot();
    });

    expect(addRoot).not.toHaveBeenCalled();
    expect(startScan).not.toHaveBeenCalled();
  });

  it("adds local root after folder pick", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 0, items: [] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);
    pickFolder.mockResolvedValue("/tmp/photos");
    addRoot.mockResolvedValue({ id: 9, path: "/tmp/photos" });
    startScan.mockResolvedValue(undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(queryAssets).toHaveBeenCalled());

    await act(async () => {
      await result.current.actions.addLocalRoot();
    });

    expect(pickFolder).toHaveBeenCalled();
    expect(addRoot).toHaveBeenCalledWith("/tmp/photos");
    expect(startScan).toHaveBeenCalledWith(9);
  });

  it("closes purge dialog via closePurgeDialog", async () => {
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    queryAssets.mockResolvedValue({ total: 1, items: [sampleCard] });
    onScanProgress.mockResolvedValue(() => undefined);
    onJobProgress.mockResolvedValue(() => undefined);
    onMessageNotify.mockResolvedValue(() => undefined);

    const { result } = renderHook(() => useLibrary());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    await act(async () => {
      await result.current.actions.selectAsset(sampleCard, false, false);
    });
    act(() => {
      result.current.actions.batchPurge();
    });
    expect(result.current.purgeDialogOpen).toBe(true);

    act(() => {
      result.current.closePurgeDialog();
    });
    expect(result.current.purgeDialogOpen).toBe(false);
    expect(result.current.purgeTargetIds).toEqual([]);
  });
});
