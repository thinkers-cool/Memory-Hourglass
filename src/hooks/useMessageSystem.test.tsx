import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { undoActivity, onMessageNotify } = vi.hoisted(() => ({
  undoActivity: vi.fn(),
  onMessageNotify: vi.fn(),
}));

vi.mock("../api/client", () => ({
  undoActivity,
  onMessageNotify,
}));

import { dismissToast, getBusy, getToast } from "../lib/message/bus";
import { useMessageSystem } from "./useMessageSystem";

describe("useMessageSystem", () => {
  beforeEach(() => {
    dismissToast();
    undoActivity.mockReset();
    undoActivity.mockResolvedValue(99);
    onMessageNotify.mockResolvedValue(() => undefined);
  });

  it("undoes activity and shows a success flash", async () => {
    const { result } = renderHook(() => useMessageSystem());

    await act(async () => {
      await result.current.undoActivity(42);
    });

    expect(undoActivity).toHaveBeenCalledWith(42);
    await waitFor(() => {
      expect(getToast()?.text_key).toBe("library:notification.undone");
    });
  });

  it("reports errors through toast", async () => {
    const { result } = renderHook(() => useMessageSystem());

    act(() => {
      result.current.reportError(new Error("save failed"));
    });

    await waitFor(() => {
      expect(result.current.notification?.text).toContain("save failed");
    });
  });

  it("wraps async work with busy state", async () => {
    const { result } = renderHook(() => useMessageSystem());
    let busyDuringWork = false;

    await act(async () => {
      await result.current.withBusy(async () => {
        busyDuringWork = getBusy();
      });
    });

    expect(busyDuringWork).toBe(true);
    expect(result.current.busy).toBe(false);
  });

  it("reports withBusy errors and rethrows", async () => {
    const { result } = renderHook(() => useMessageSystem());

    await expect(
      result.current.withBusy(async () => {
        throw new Error("busy failed");
      }),
    ).rejects.toThrow("busy failed");

    await waitFor(() => {
      expect(result.current.notification?.text).toContain("busy failed");
    });
  });

  it("sets and clears notifications", async () => {
    const { result } = renderHook(() => useMessageSystem());

    act(() => {
      result.current.setNotification({
        kind: "info",
        text: "Saved",
      });
    });
    await waitFor(() => {
      expect(result.current.notification?.text).toBe("Saved");
    });

    act(() => {
      result.current.setNotification(null);
    });
    await waitFor(() => {
      expect(result.current.notification).toBeNull();
    });
  });

  it("dispatches custom messages", async () => {
    const { result } = renderHook(() => useMessageSystem());

    act(() => {
      result.current.dispatch({
        kind: "success",
        text: "Done",
        duration_ms: 1000,
      });
    });

    await waitFor(() => {
      expect(result.current.notification?.text).toBe("Done");
    });
  });

  it("handles backend message notify events", async () => {
    let notifyHandler:
      | ((envelope: {
          kind: string;
          source: string;
          text_key: string;
          text_params: Record<string, string | number>;
          actions: [];
        }) => void)
      | undefined;
    onMessageNotify.mockImplementation(async (handler) => {
      notifyHandler = handler;
      return () => undefined;
    });

    const { result } = renderHook(() => useMessageSystem());
    await waitFor(() => expect(onMessageNotify).toHaveBeenCalled());

    act(() => {
      notifyHandler?.({
        kind: "warning",
        source: "library",
        text_key: "library:notification.saved",
        text_params: {},
        actions: [],
      });
    });

    await waitFor(() => {
      expect(result.current.notification?.kind).toBe("info");
    });
  });
});
