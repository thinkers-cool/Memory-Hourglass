import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { usePopoverDismiss } from "./usePopoverDismiss";

describe("usePopoverDismiss", () => {
  it("closes on outside click and escape", () => {
    const onClose = vi.fn();
    const container = document.createElement("div");
    const anchor = document.createElement("button");
    document.body.appendChild(container);
    document.body.appendChild(anchor);
    const containerRef = { current: container };
    const anchorRef = { current: anchor };

    renderHook(() =>
      usePopoverDismiss({ open: true, onClose, containerRef, anchorRef }),
    );

    document.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    expect(onClose).toHaveBeenCalledTimes(1);

    onClose.mockClear();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(onClose).toHaveBeenCalledTimes(1);

    onClose.mockClear();
    container.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    expect(onClose).not.toHaveBeenCalled();

    container.remove();
    anchor.remove();
  });

  it("does nothing when closed", () => {
    const onClose = vi.fn();
    const containerRef = { current: document.createElement("div") };
    renderHook(() =>
      usePopoverDismiss({ open: false, onClose, containerRef }),
    );
    document.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    expect(onClose).not.toHaveBeenCalled();
  });
});
