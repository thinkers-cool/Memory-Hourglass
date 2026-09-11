import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const jobProgressHandler = vi.fn();

vi.mock("../../api/client", () => ({
  onJobProgress: vi.fn((handler: (progress: unknown) => void) => {
    jobProgressHandler.mockImplementation(handler);
    return Promise.resolve(() => undefined);
  }),
  startExport: vi.fn(),
}));

import * as api from "../../api/client";
import { useLibraryExport } from "./useLibraryExport";

describe("useLibraryExport", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    jobProgressHandler.mockReset();
  });

  it("opens export dialog with asset ids", () => {
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    act(() => {
      result.current.openExport([1, 2]);
    });

    expect(result.current.exportDialog.open).toBe(true);
    expect(result.current.exportDialog.assetIds).toEqual([1, 2]);
  });

  it("ignores empty export open", () => {
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    act(() => {
      result.current.openExport([]);
    });

    expect(result.current.exportDialog.open).toBe(false);
  });

  it("runs export and tracks progress", async () => {
    vi.mocked(api.startExport).mockResolvedValue(undefined);
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    await act(async () => {
      await result.current.runExport([1], "/tmp/out", {
        flat: true,
        rename_template: undefined,
        format: undefined,
      });
    });

    expect(api.startExport).toHaveBeenCalled();
    expect(result.current.exportDialog.progress?.phase).toBe("started");
  });

  it("handles job progress and completion notification", async () => {
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    act(() => {
      result.current.openExport([1]);
    });

    const jobId = Date.now();
    act(() => {
      result.current.updateExportDialog({ jobId });
    });

    await waitFor(() => {
      expect(jobProgressHandler).toBeTruthy();
    });

    act(() => {
      jobProgressHandler({
        job_id: String(jobId),
        done: 1,
        total: 1,
        phase: "completed",
        message: "done",
      });
    });

    expect(setNotification).toHaveBeenCalled();
    expect(result.current.exportDialog.jobId).toBeNull();
  });

  it("ignores progress for other jobs", async () => {
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    act(() => {
      result.current.updateExportDialog({ jobId: 42, progress: null });
    });

    await waitFor(() => {
      expect(jobProgressHandler).toBeTruthy();
    });

    act(() => {
      jobProgressHandler({
        job_id: "99",
        done: 1,
        total: 1,
        phase: "running",
        message: "running",
      });
    });

    expect(result.current.exportDialog.jobId).toBe(42);
    expect(setNotification).not.toHaveBeenCalled();
  });

  it("requires destination before starting export", async () => {
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    act(() => {
      result.current.openExport([1]);
    });

    await act(async () => {
      await result.current.startExportFromDialog();
    });

    expect(setNotification).toHaveBeenCalled();
    expect(api.startExport).not.toHaveBeenCalled();
  });

  it("reports export start errors", async () => {
    vi.mocked(api.startExport).mockRejectedValue(new Error("fail"));
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    act(() => {
      result.current.openExport([1]);
    });
    act(() => {
      result.current.updateExportDialog({ destination: "/tmp/out" });
    });
    await vi.waitFor(() => {
      expect(result.current.exportDialog.destination).toBe("/tmp/out");
    });
    await act(async () => {
      await result.current.startExportFromDialog();
    });

    expect(setNotification).toHaveBeenCalled();
    expect(api.startExport).toHaveBeenCalled();
  });

  it("updates progress while export is running", async () => {
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    act(() => {
      result.current.updateExportDialog({ jobId: 42, progress: null });
    });

    await waitFor(() => {
      expect(jobProgressHandler).toBeTruthy();
    });

    act(() => {
      jobProgressHandler({
        job_id: "42",
        done: 1,
        total: 2,
        phase: "running",
        message: "running",
      });
    });

    expect(result.current.exportDialog.jobId).toBe(42);
    expect(result.current.exportDialog.progress?.phase).toBe("running");
    expect(setNotification).not.toHaveBeenCalled();
  });

  it("ignores job progress listener registration failures", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
    vi.mocked(api.onJobProgress).mockRejectedValueOnce(new Error("listener failed"));
    renderHook(() => useLibraryExport(vi.fn()));
    await waitFor(() => {
      expect(errorSpy).toHaveBeenCalled();
    });
    errorSpy.mockRestore();
  });

  it("closes export dialog", () => {
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    act(() => {
      result.current.openExport([1]);
      result.current.closeExportDialog();
    });

    expect(result.current.exportDialog.open).toBe(false);
    expect(result.current.exportDialog.jobId).toBeNull();
  });

  it("requires asset ids before starting export", async () => {
    const setNotification = vi.fn();
    const { result } = renderHook(() => useLibraryExport(setNotification));

    act(() => {
      result.current.openExport([1]);
      result.current.updateExportDialog({ destination: "/tmp/out", assetIds: [] });
    });

    await act(async () => {
      await result.current.startExportFromDialog();
    });

    expect(setNotification).toHaveBeenCalled();
    expect(api.startExport).not.toHaveBeenCalled();
  });
});
