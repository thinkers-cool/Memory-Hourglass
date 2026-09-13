import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api/client", () => ({
  listRootStats: vi.fn(),
  listAlbums: vi.fn(),
  listSmartCollections: vi.fn(),
  listTags: vi.fn(),
  countAssets: vi.fn(),
  queryAssets: vi.fn(),
  onScanProgress: vi.fn(() => Promise.resolve(() => undefined)),
  onScanThumbs: vi.fn(() => Promise.resolve(() => undefined)),
  getAsset: vi.fn(),
  resumePendingScans: vi.fn(() => Promise.resolve()),
}));

import * as api from "../../api/client";
import { useLibraryQuery } from "./useLibraryQuery";

describe("useLibraryQuery", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.listRootStats).mockResolvedValue([]);
    vi.mocked(api.listAlbums).mockResolvedValue([]);
    vi.mocked(api.listSmartCollections).mockResolvedValue([]);
    vi.mocked(api.listTags).mockResolvedValue([]);
    vi.mocked(api.countAssets).mockResolvedValue(0);
    vi.mocked(api.queryAssets).mockResolvedValue({ items: [], total: 0 });
  });

  it("reports meta load failures", async () => {
    vi.mocked(api.listRootStats).mockRejectedValue(new Error("meta failed"));
    const setNotification = vi.fn();
    renderHook(() =>
      useLibraryQuery({ current: null }, vi.fn(), setNotification),
    );
    await waitFor(() => {
      expect(setNotification).toHaveBeenCalled();
    });
  });

  it("reports grid load failures", async () => {
    vi.mocked(api.queryAssets).mockRejectedValue(new Error("grid failed"));
    const setNotification = vi.fn();
    renderHook(() =>
      useLibraryQuery({ current: null }, vi.fn(), setNotification),
    );
    await waitFor(() => {
      expect(setNotification).toHaveBeenCalled();
    });
  });

  it("ignores stale grid responses", async () => {
    let resolveFirst:
      ((value: { items: []; total: number }) => void) | undefined;
    const first = new Promise<{ items: []; total: number }>((resolve) => {
      resolveFirst = resolve;
    });
    vi.mocked(api.queryAssets)
      .mockReturnValueOnce(first)
      .mockResolvedValue({
        items: [{ id: 2, file_name: "new.jpg" } as never],
        total: 1,
      });

    const setNotification = vi.fn();
    const { result } = renderHook(() =>
      useLibraryQuery({ current: null }, vi.fn(), setNotification),
    );

    await waitFor(() => {
      expect(api.queryAssets).toHaveBeenCalled();
    });

    await act(async () => {
      result.current.setFilterBar((prev) => ({ ...prev, camera: "Canon" }));
    });

    await waitFor(() => {
      expect(api.queryAssets).toHaveBeenCalledTimes(2);
    });

    await act(async () => {
      resolveFirst?.({
        items: [{ id: 1, file_name: "old.jpg" } as never],
        total: 1,
      });
    });

    await waitFor(() => {
      expect(result.current.items).toEqual([{ id: 2, file_name: "new.jpg" }]);
    });
  });

  it("schedules meta refresh during indexing progress", async () => {
    let scanHandler:
      | ((progress: {
          root_id: number;
          stage: string;
          scanned: number;
          indexed: number;
        }) => void)
      | undefined;

    vi.mocked(api.onScanProgress).mockImplementation(async (handler) => {
      scanHandler = handler;
      return () => undefined;
    });

    renderHook(() => useLibraryQuery({ current: null }, vi.fn(), vi.fn()));
    await waitFor(() => expect(api.onScanProgress).toHaveBeenCalled());

    vi.mocked(api.listTags).mockClear();
    vi.useFakeTimers();
    await act(async () => {
      scanHandler?.({
        root_id: 1,
        stage: "indexing",
        scanned: 10,
        indexed: 4,
      });
      vi.advanceTimersByTime(5000);
    });
    vi.useRealTimers();

    await waitFor(() => expect(api.listTags).toHaveBeenCalled());
  });

  it("skips meta refresh timers for non-catalog stages", async () => {
    let scanHandler:
      | ((progress: {
          root_id: number;
          stage: string;
          scanned: number;
          indexed: number;
        }) => void)
      | undefined;

    vi.mocked(api.onScanProgress).mockImplementation(async (handler) => {
      scanHandler = handler;
      return () => undefined;
    });

    renderHook(() => useLibraryQuery({ current: null }, vi.fn(), vi.fn()));
    await waitFor(() => expect(api.onScanProgress).toHaveBeenCalled());

    const setTimeoutSpy = vi.spyOn(window, "setTimeout");
    scanHandler?.({
      root_id: 1,
      stage: "scanning",
      scanned: 3,
      indexed: 0,
    });
    const metaRefreshTimers = setTimeoutSpy.mock.calls.filter(
      ([, delay]) => delay === 5000,
    );
    expect(metaRefreshTimers).toHaveLength(0);
    setTimeoutSpy.mockRestore();
  });

  it("ignores stale grid errors", async () => {
    let rejectFirst: ((error: Error) => void) | undefined;
    const first = new Promise<{ items: []; total: number }>((_, reject) => {
      rejectFirst = reject;
    });
    vi.mocked(api.queryAssets)
      .mockReturnValueOnce(first)
      .mockResolvedValue({ items: [], total: 0 });

    const setNotification = vi.fn();
    const { result } = renderHook(() =>
      useLibraryQuery({ current: null }, vi.fn(), setNotification),
    );

    await waitFor(() => {
      expect(api.queryAssets).toHaveBeenCalled();
    });

    await act(async () => {
      result.current.setFilterBar((prev) => ({ ...prev, camera: "Nikon" }));
    });

    await waitFor(() => {
      expect(api.queryAssets).toHaveBeenCalledTimes(2);
    });

    await act(async () => {
      rejectFirst?.(new Error("stale grid failed"));
    });

    expect(setNotification).not.toHaveBeenCalled();
  });
});
