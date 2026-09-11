import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useAnchoredPopoverPlacement } from "./useAnchoredPopover";

describe("useAnchoredPopoverPlacement", () => {
  it("returns null when closed or anchor missing", () => {
    const anchorRef = { current: null };
    const { result } = renderHook(() =>
      useAnchoredPopoverPlacement({ open: false, anchorRef }),
    );
    expect(result.current).toBeNull();
  });

  it("computes placement and reacts to resize", () => {
    const anchor = document.createElement("button");
    const panel = document.createElement("div");
    panel.setAttribute("data-library-panel", "");
    panel.appendChild(anchor);
    document.body.appendChild(panel);
    anchor.getBoundingClientRect = () =>
      ({
        left: 10,
        top: 20,
        bottom: 40,
        right: 50,
        width: 40,
        height: 20,
        x: 10,
        y: 20,
        toJSON: () => ({}),
      }) as DOMRect;
    panel.getBoundingClientRect = () =>
      ({
        left: 0,
        top: 0,
        bottom: 400,
        right: 400,
        width: 400,
        height: 400,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }) as DOMRect;

    const anchorRef = { current: anchor };
    const { result, unmount } = renderHook(() =>
      useAnchoredPopoverPlacement({ open: true, anchorRef }),
    );

    act(() => {
      window.dispatchEvent(new Event("resize"));
      window.dispatchEvent(new Event("scroll"));
    });

    expect(result.current).not.toBeNull();
    unmount();
    panel.remove();
  });

  it("uses window bounds when boundary is missing", () => {
    const anchor = document.createElement("button");
    document.body.appendChild(anchor);
    anchor.getBoundingClientRect = () =>
      ({
        left: 10,
        top: 20,
        bottom: 40,
        right: 50,
        width: 40,
        height: 20,
        x: 10,
        y: 20,
        toJSON: () => ({}),
      }) as DOMRect;
    const anchorRef = { current: anchor };
    const { result, unmount } = renderHook(() =>
      useAnchoredPopoverPlacement({
        open: true,
        anchorRef,
        boundarySelector: "[data-missing-boundary]",
      }),
    );
    expect(result.current).not.toBeNull();
    unmount();
    anchor.remove();
  });
});
