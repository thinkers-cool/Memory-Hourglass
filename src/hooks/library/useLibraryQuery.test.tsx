import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AssetCard, AssetDetail } from "../../types";

const {
  listRootStats,
  listAlbums,
  listSmartCollections,
  listTags,
  countAssets,
  queryAssets,
  onScanProgress,
  getAsset,
} = vi.hoisted(() => ({
  listRootStats: vi.fn(),
  listAlbums: vi.fn(),
  listSmartCollections: vi.fn(),
  listTags: vi.fn(),
  countAssets: vi.fn(),
  queryAssets: vi.fn(),
  onScanProgress: vi.fn(),
  getAsset: vi.fn(),
}));

vi.mock("../../api/client", () => ({
  listRootStats,
  listAlbums,
  listSmartCollections,
  listTags,
  countAssets,
  queryAssets,
  onScanProgress,
  getAsset,
}));

import { useLibraryQuery } from "./useLibraryQuery";

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

const sampleDetail: AssetDetail = {
  asset: {
    id: 1,
    root_id: 1,
    rel_path: "a.jpg",
    file_name: "a.jpg",
    ext: "jpg",
    kind: "image",
    size: 1,
    mtime_ns: 0,
    sync_state: "ok",
  },
  meta: null,
  abs_path: "/tmp/a.jpg",
  display_path: "/tmp/a.jpg",
  tag_ids: [],
  album_ids: [],
  raw_tags: [],
  links: [],
  duplicates: [],
};

describe("useLibraryQuery", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listRootStats.mockResolvedValue([]);
    listAlbums.mockResolvedValue([]);
    listSmartCollections.mockResolvedValue([]);
    listTags.mockResolvedValue([]);
    countAssets.mockResolvedValue(0);
    queryAssets.mockResolvedValue({ total: 1, items: [sampleCard] });
    getAsset.mockResolvedValue(sampleDetail);
    onScanProgress.mockResolvedValue(() => undefined);
  });

  it("loads metadata and grid on mount", async () => {
    const selectedIdRef = { current: null as number | null };
    const setDetail = vi.fn();
    const setNotification = vi.fn();

    const { result } = renderHook(() =>
      useLibraryQuery(selectedIdRef, setDetail, setNotification),
    );

    await waitFor(() => expect(result.current.items).toHaveLength(1));
    expect(listRootStats).toHaveBeenCalled();
    expect(queryAssets).toHaveBeenCalled();
  });

  it("loads more items and reports errors", async () => {
    const selectedIdRef = { current: null as number | null };
    const setDetail = vi.fn();
    const setNotification = vi.fn();

    queryAssets
      .mockResolvedValueOnce({ total: 3, items: [sampleCard] })
      .mockRejectedValueOnce(new Error("load failed"));

    const { result } = renderHook(() =>
      useLibraryQuery(selectedIdRef, setDetail, setNotification),
    );

    await waitFor(() => expect(result.current.hasMore).toBe(true));

    await act(async () => {
      await result.current.loadMore();
    });
    expect(setNotification).toHaveBeenCalled();
  });

  it("handles scan progress lifecycle", async () => {
    let scanHandler:
      | ((progress: {
          root_id: number;
          stage: string;
          scanned: number;
          indexed: number;
        }) => void)
      | undefined;

    onScanProgress.mockImplementation(async (handler) => {
      scanHandler = handler;
      return () => undefined;
    });

    const selectedIdRef = { current: 1 as number | null };
    const setDetail = vi.fn();
    const setNotification = vi.fn();

    const { result } = renderHook(() =>
      useLibraryQuery(selectedIdRef, setDetail, setNotification),
    );
    await waitFor(() => expect(onScanProgress).toHaveBeenCalled());

    await act(async () => {
      scanHandler?.({
        root_id: 1,
        stage: "cataloging",
        scanned: 10,
        indexed: 5,
      });
    });
    expect(result.current.scanStatus).toContain("cataloging");

    vi.useFakeTimers();
    await act(async () => {
      scanHandler?.({
        root_id: 1,
        stage: "indexing",
        scanned: 10,
        indexed: 8,
      });
      vi.advanceTimersByTime(5000);
    });
    vi.useRealTimers();

    listRootStats.mockClear();
    await act(async () => {
      scanHandler?.({
        root_id: 1,
        stage: "done",
        scanned: 10,
        indexed: 10,
      });
    });

    await waitFor(() => expect(listRootStats).toHaveBeenCalled());
    await waitFor(() => expect(getAsset).toHaveBeenCalledWith(1));
  });

  it("appends unique items when loading more", async () => {
    const selectedIdRef = { current: null as number | null };
    const setDetail = vi.fn();
    const setNotification = vi.fn();
    const secondCard = { ...sampleCard, id: 2, file_name: "b.jpg" };

    queryAssets
      .mockResolvedValueOnce({ total: 3, items: [sampleCard] })
      .mockResolvedValueOnce({ total: 3, items: [sampleCard, secondCard] });

    const { result } = renderHook(() =>
      useLibraryQuery(selectedIdRef, setDetail, setNotification),
    );

    await waitFor(() => expect(result.current.hasMore).toBe(true));

    await act(async () => {
      await result.current.loadMore();
    });

    expect(result.current.items).toHaveLength(2);
    expect(setNotification).not.toHaveBeenCalled();
  });

  it("refreshes grid during cataloging progress", async () => {
    let scanHandler:
      | ((progress: {
          root_id: number;
          stage: string;
          scanned: number;
          indexed: number;
        }) => void)
      | undefined;

    onScanProgress.mockImplementation(async (handler) => {
      scanHandler = handler;
      return () => undefined;
    });

    const selectedIdRef = { current: null as number | null };
    const setDetail = vi.fn();
    const setNotification = vi.fn();

    renderHook(() => useLibraryQuery(selectedIdRef, setDetail, setNotification));
    await waitFor(() => expect(onScanProgress).toHaveBeenCalled());

    queryAssets.mockClear();
    vi.useFakeTimers();
    await act(async () => {
      scanHandler?.({
        root_id: 1,
        stage: "cataloging",
        scanned: 10,
        indexed: 2,
      });
      vi.advanceTimersByTime(8000);
    });
    vi.useRealTimers();

    await waitFor(() => expect(queryAssets).toHaveBeenCalled());
  });

  it("skips loadMore when nothing left to load", async () => {
    const selectedIdRef = { current: null as number | null };
    const setDetail = vi.fn();
    const setNotification = vi.fn();

    queryAssets.mockResolvedValue({ total: 1, items: [sampleCard] });

    const { result } = renderHook(() =>
      useLibraryQuery(selectedIdRef, setDetail, setNotification),
    );

    await waitFor(() => expect(result.current.hasMore).toBe(false));

    await act(async () => {
      await result.current.loadMore();
    });
    expect(queryAssets).toHaveBeenCalledTimes(1);
  });
});
