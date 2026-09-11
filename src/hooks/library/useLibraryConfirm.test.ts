import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useLibraryConfirm } from "./useLibraryConfirm";

describe("useLibraryConfirm", () => {
  it("opens and closes confirm dialog", () => {
    const { result } = renderHook(() => useLibraryConfirm());
    const onConfirm = vi.fn();

    act(() => {
      result.current.requestConfirm("Title", "Message", onConfirm);
    });
    expect(result.current.confirmDialog.open).toBe(true);
    expect(result.current.confirmDialog.title).toBe("Title");
    expect(result.current.confirmDialog.message).toBe("Message");

    act(() => {
      result.current.closeConfirmDialog();
    });
    expect(result.current.confirmDialog.open).toBe(false);
  });

  it("stores onConfirm handler in dialog state", async () => {
    const { result } = renderHook(() => useLibraryConfirm());
    const onConfirm = vi.fn().mockResolvedValue(undefined);

    act(() => {
      result.current.requestConfirm("Title", "Message", onConfirm);
    });

    await act(async () => {
      await result.current.confirmDialog.onConfirm();
    });
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("uses noop confirm handler before dialog opens", async () => {
    const { result } = renderHook(() => useLibraryConfirm());
    await act(async () => {
      await result.current.confirmDialog.onConfirm();
    });
    expect(result.current.confirmDialog.open).toBe(false);
  });
});
