import { act, createEvent, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ResizableSplit, ResizableTrailingPanel } from "./ResizableSplit";

function fireWidthTransitionEnd(element: Element) {
  const event = createEvent.transitionEnd(element, { propertyName: "width" });
  Object.defineProperty(event, "propertyName", { value: "width" });
  fireEvent(element, event);
}

function mockRect(element: HTMLElement, rect: Partial<DOMRect>) {
  element.getBoundingClientRect = () =>
    ({
      x: 0,
      y: 0,
      top: 0,
      left: 0,
      right: 800,
      bottom: 400,
      width: 800,
      height: 400,
      toJSON: () => ({}),
      ...rect,
    }) as DOMRect;
}

describe("ResizableSplit", () => {
  it("renders leading and trailing panels", () => {
    render(
      <div style={{ width: 800, height: 400 }}>
        <ResizableSplit
          leading={<div>Leading</div>}
          trailing={<div>Trailing</div>}
          leadingWidth={240}
          onLeadingWidthChange={vi.fn()}
          minLeading={180}
          minTrailing={180}
        />
      </div>,
    );
    expect(screen.getByText("Leading")).toBeInTheDocument();
    expect(screen.getByText("Trailing")).toBeInTheDocument();
  });

  it("resizes leading panel on drag and clamps to min width", () => {
    const onLeadingWidthChange = vi.fn();
    const { container } = render(
      <div style={{ width: 800, height: 400 }}>
        <ResizableSplit
          leading={<div>Leading</div>}
          trailing={<div>Trailing</div>}
          leadingWidth={240}
          onLeadingWidthChange={onLeadingWidthChange}
          minLeading={180}
          minTrailing={180}
        />
      </div>,
    );
    const splitRoot = container.querySelector(".flex.min-h-0.min-w-0.flex-1") as HTMLElement;
    mockRect(splitRoot, { width: 800, left: 0, right: 800 });
    const divider = container.querySelector(".cursor-col-resize") as HTMLElement;
    divider.setPointerCapture = vi.fn();
    divider.releasePointerCapture = vi.fn();

    fireEvent.pointerDown(divider, { clientX: 240, pointerId: 1 });
    fireEvent.pointerMove(window, { clientX: 300, pointerId: 1 });
    expect(onLeadingWidthChange).toHaveBeenCalledWith(300);

    fireEvent.pointerMove(window, { clientX: 20, pointerId: 1 });
    expect(onLeadingWidthChange).toHaveBeenCalledWith(180);

    fireEvent.pointerUp(window, { pointerId: 1 });
    expect(splitRoot.className).not.toContain("select-none");
  });
});

describe("ResizableTrailingPanel", () => {
  it("renders main content and side panel when shown", () => {
    render(
      <div style={{ width: 800, height: 400 }}>
        <ResizableTrailingPanel
          main={<div>Main</div>}
          side={<div>Side</div>}
          sideWidth={280}
          onSideWidthChange={vi.fn()}
          minMain={280}
          minSide={240}
          showSide
        />
      </div>,
    );
    expect(screen.getByText("Main")).toBeInTheDocument();
    expect(screen.getByText("Side")).toBeInTheDocument();
  });

  it("resizes side panel on drag", () => {
    const onSideWidthChange = vi.fn();
    const { container } = render(
      <div style={{ width: 800, height: 400 }}>
        <ResizableTrailingPanel
          main={<div>Main</div>}
          side={<div>Side</div>}
          sideWidth={280}
          onSideWidthChange={onSideWidthChange}
          minMain={280}
          minSide={240}
          showSide
        />
      </div>,
    );
    const panelRoot = container.querySelector(".relative.z-0.flex") as HTMLElement;
    mockRect(panelRoot, { width: 800, left: 0, right: 800 });
    const divider = container.querySelector(".cursor-col-resize") as HTMLElement;
    divider.setPointerCapture = vi.fn();

    fireEvent.pointerDown(divider, { clientX: 520, pointerId: 2 });
    fireEvent.pointerMove(window, { clientX: 480, pointerId: 2 });
    expect(onSideWidthChange).toHaveBeenCalledWith(320);

    fireEvent.pointerUp(window, { pointerId: 2 });
  });

  it("animates side panel open when showSide becomes true", () => {
    const rafSpy = vi
      .spyOn(window, "requestAnimationFrame")
      .mockImplementation((callback: FrameRequestCallback) => {
        callback(0);
        return 1;
      });
    const { rerender } = render(
      <div style={{ width: 800, height: 400 }}>
        <ResizableTrailingPanel
          main={<div>Main</div>}
          side={<div>Side</div>}
          sideWidth={280}
          onSideWidthChange={vi.fn()}
          minMain={280}
          minSide={240}
          showSide={false}
        />
      </div>,
    );
    expect(screen.queryByText("Side")).not.toBeInTheDocument();

    rerender(
      <div style={{ width: 800, height: 400 }}>
        <ResizableTrailingPanel
          main={<div>Main</div>}
          side={<div>Side</div>}
          sideWidth={280}
          onSideWidthChange={vi.fn()}
          minMain={280}
          minSide={240}
          showSide
        />
      </div>,
    );

    expect(screen.getByText("Side")).toBeInTheDocument();
    expect(rafSpy).toHaveBeenCalled();
    rafSpy.mockRestore();
  });

  it("hides side panel when showSide is false", async () => {
    const { rerender } = render(
      <div style={{ width: 800, height: 400 }}>
        <ResizableTrailingPanel
          main={<div>Main</div>}
          side={<div>Side</div>}
          sideWidth={280}
          onSideWidthChange={vi.fn()}
          minMain={280}
          minSide={240}
          showSide
        />
      </div>,
    );
    expect(screen.getByText("Side")).toBeInTheDocument();

    await act(async () => {
      rerender(
        <div style={{ width: 800, height: 400 }}>
          <ResizableTrailingPanel
            main={<div>Main</div>}
            side={<div>Side</div>}
            sideWidth={280}
            onSideWidthChange={vi.fn()}
            minMain={280}
            minSide={240}
            showSide={false}
          />
        </div>,
      );
    });

    const sideContainer = screen.getByText("Side").parentElement?.parentElement;
    await act(async () => {
      fireWidthTransitionEnd(sideContainer!);
    });
    await waitFor(() => {
      expect(screen.queryByText("Side")).not.toBeInTheDocument();
    });
  });

  it("ignores unrelated transition events", () => {
    const { container } = render(
      <div style={{ width: 800, height: 400 }}>
        <ResizableTrailingPanel
          main={<div>Main</div>}
          side={<div>Side</div>}
          sideWidth={280}
          onSideWidthChange={vi.fn()}
          minMain={280}
          minSide={240}
          showSide={false}
        />
      </div>,
    );
    const sideContainer = container.querySelector(".flex.min-h-0.shrink-0");
    if (sideContainer) {
      fireEvent.transitionEnd(sideContainer, { propertyName: "opacity" });
    }
  });

  it("ignores non-width transition events while side panel is open", () => {
    render(
      <div style={{ width: 800, height: 400 }}>
        <ResizableTrailingPanel
          main={<div>Main</div>}
          side={<div>Side</div>}
          sideWidth={280}
          onSideWidthChange={vi.fn()}
          minMain={280}
          minSide={240}
          showSide
        />
      </div>,
    );
    const sideContainer = screen.getByText("Side").parentElement?.parentElement;
    const event = createEvent.transitionEnd(sideContainer!, { propertyName: "opacity" });
    Object.defineProperty(event, "propertyName", { value: "opacity" });
    fireEvent(sideContainer!, event);
    expect(screen.getByText("Side")).toBeInTheDocument();
  });
});
