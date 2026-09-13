import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { usePanelOrder } from "./usePanelOrder";

describe("usePanelOrder", () => {
  it("toggles section order and persists to storage", () => {
    const { result } = renderHook(() => usePanelOrder("ws-panel-order"));
    expect(result.current.store.roots).toBe("name-asc");

    act(() => {
      result.current.toggleSection("roots");
    });
    expect(result.current.store.roots).toBe("name-desc");

    act(() => {
      result.current.toggleSection("albums");
    });
    expect(result.current.store.albums).toBe("name-desc");

    act(() => {
      result.current.toggleSection("tags");
    });
    expect(result.current.store.tags).toBe("name-desc");
  });

  it("reloads stored order when workspace changes", () => {
    const { result, rerender } = renderHook(
      ({ workspaceId }) => usePanelOrder(workspaceId),
      { initialProps: { workspaceId: "ws-a" } },
    );

    act(() => {
      result.current.toggleSection("roots");
    });
    expect(result.current.store.roots).toBe("name-desc");

    rerender({ workspaceId: "ws-b" });
    expect(result.current.store.roots).toBe("name-asc");
  });
});
