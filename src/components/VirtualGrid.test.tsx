import { StrictMode } from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  GRID_CARD_DOUBLE_CLICK_WINDOW_MS,
  resetGridCardClickState,
} from "../lib/gridCardClick";
import { sampleCard } from "../test/fixtures";
import { EMPTY_STAMP_CONFIG } from "../lib/stamp";
import { VirtualGrid } from "./VirtualGrid";

const scrollToIndex = vi.fn();

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: (options: {
    getScrollElement?: () => Element | null;
    estimateSize?: (index: number) => number;
  }) => {
    options.getScrollElement?.();
    options.estimateSize?.(0);
    return {
      getVirtualItems: () => [{ index: 0, start: 0, size: 140, key: "row-0" }],
      getTotalSize: () => 140,
      measure: vi.fn(),
      scrollToIndex,
    };
  },
}));

describe("VirtualGrid", () => {
  afterEach(() => {
    resetGridCardClickState();
    scrollToIndex.mockClear();
    vi.useRealTimers();
  });

  function renderGrid(
    overrides: Partial<Parameters<typeof VirtualGrid>[0]> = {},
  ) {
    const props = {
      items: [sampleCard],
      selectedId: null as number | null,
      selectedIds: new Set<number>(),
      columnCount: 1,
      stampConfig: EMPTY_STAMP_CONFIG,
      stampArmed: false,
      stampMatchedIds: new Set<number>(),
      onSelect: vi.fn(),
      onOpenFullView: vi.fn(),
      ...overrides,
    };
    const view = render(
      <div style={{ width: 800, height: 600 }}>
        <VirtualGrid {...props} />
      </div>,
    );
    return { ...props, ...view };
  }

  it("renders grid cards and handles selection", () => {
    vi.useFakeTimers();
    const { onSelect } = renderGrid();
    fireEvent.click(screen.getByRole("button"));
    expect(onSelect).not.toHaveBeenCalled();
    vi.advanceTimersByTime(GRID_CARD_DOUBLE_CLICK_WINDOW_MS);
    expect(onSelect).toHaveBeenCalledWith(sampleCard, false, false);
  });

  it("opens full view on double click", () => {
    const { onOpenFullView } = renderGrid();
    const button = screen.getByRole("button");
    fireEvent.click(button);
    fireEvent.click(button);
    expect(onOpenFullView).toHaveBeenCalledWith(sampleCard);
  });

  it("scrolls selected row into view when selection changes", () => {
    const { rerender } = renderGrid({ selectedId: null });
    rerender(
      <div style={{ width: 800, height: 600 }}>
        <VirtualGrid
          items={[sampleCard]}
          selectedId={1}
          selectedIds={new Set([1])}
          columnCount={1}
          stampConfig={EMPTY_STAMP_CONFIG}
          stampArmed={false}
          stampMatchedIds={new Set()}
          onSelect={vi.fn()}
          onOpenFullView={vi.fn()}
        />
      </div>,
    );
    expect(scrollToIndex).toHaveBeenCalled();
  });

  it("does not load more when scrolled away from bottom", () => {
    const onLoadMore = vi.fn();
    const { container } = renderGrid({
      hasMore: true,
      loadingMore: false,
      onLoadMore,
    });
    const scrollArea = container.querySelector(".grid-canvas") as HTMLElement;
    Object.defineProperty(scrollArea, "scrollHeight", {
      value: 2000,
      configurable: true,
    });
    Object.defineProperty(scrollArea, "clientHeight", {
      value: 600,
      configurable: true,
    });
    Object.defineProperty(scrollArea, "scrollTop", {
      value: 0,
      writable: true,
      configurable: true,
    });
    fireEvent.scroll(scrollArea);
    expect(onLoadMore).not.toHaveBeenCalled();
  });

  it("loads more when scrolled near bottom", () => {
    const onLoadMore = vi.fn();
    const { container } = renderGrid({
      hasMore: true,
      loadingMore: false,
      onLoadMore,
    });
    const scrollArea = container.querySelector(".grid-canvas") as HTMLElement;
    Object.defineProperty(scrollArea, "scrollHeight", {
      value: 2000,
      configurable: true,
    });
    Object.defineProperty(scrollArea, "clientHeight", {
      value: 600,
      configurable: true,
    });
    Object.defineProperty(scrollArea, "scrollTop", {
      value: 1400,
      writable: true,
      configurable: true,
    });
    fireEvent.scroll(scrollArea);
    expect(onLoadMore).toHaveBeenCalled();
  });

  it("removes scroll listener on unmount", () => {
    const { unmount } = renderGrid({
      hasMore: true,
      loadingMore: false,
      onLoadMore: vi.fn(),
    });
    unmount();
  });

  it("shows loading spinner while loading more", () => {
    renderGrid({ loadingMore: true, hasMore: true, onLoadMore: vi.fn() });
    expect(document.querySelector(".loading-spinner")).toBeInTheDocument();
  });

  it("adds bottom padding when items are selected", () => {
    const { container } = renderGrid({ selectedIds: new Set([1]) });
    expect(container.querySelector(".pb-24")).toBeInTheDocument();
  });

  it("updates layout from ResizeObserver callback", () => {
    let triggerResize: ((entries: ResizeObserverEntry[]) => void) | undefined;
    vi.stubGlobal(
      "ResizeObserver",
      class {
        constructor(callback: ResizeObserverCallback) {
          triggerResize = (entries) =>
            callback(entries, this as ResizeObserver);
        }
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );
    renderGrid({ columnCount: 4 });
    act(() => {
      triggerResize?.([{ contentRect: { width: 640 } } as ResizeObserverEntry]);
    });
    act(() => {
      triggerResize?.([]);
    });
    expect(document.querySelector(".grid-canvas")).toBeInTheDocument();
  });

  it("skips load more when guards block scrolling", () => {
    const onLoadMore = vi.fn();
    const { container } = renderGrid({
      hasMore: true,
      loadingMore: true,
      onLoadMore,
    });
    const scrollArea = container.querySelector(".grid-canvas") as HTMLElement;
    Object.defineProperty(scrollArea, "scrollHeight", {
      value: 2000,
      configurable: true,
    });
    Object.defineProperty(scrollArea, "clientHeight", {
      value: 600,
      configurable: true,
    });
    Object.defineProperty(scrollArea, "scrollTop", {
      value: 1400,
      writable: true,
      configurable: true,
    });
    fireEvent.scroll(scrollArea);
    expect(onLoadMore).not.toHaveBeenCalled();
  });

  it("renders stamp indicators when stamp mode is armed", () => {
    renderGrid({
      stampArmed: true,
      stampMatchedIds: new Set([sampleCard.id]),
      stampConfig: { rating: 4, tag_ids: [], album_ids: [] },
    });
    expect(screen.getByRole("button")).toBeInTheDocument();
  });

  it("renders empty cells when column count exceeds items", () => {
    const { container } = renderGrid({ columnCount: 2, items: [sampleCard] });
    const row = container.querySelector(".absolute.top-0.left-0.grid");
    expect(row?.children.length).toBe(2);
  });

  it("scrolls selected row when column count changes", () => {
    const { rerender } = renderGrid({
      selectedId: 1,
      selectedIds: new Set([1]),
    });
    scrollToIndex.mockClear();
    rerender(
      <div style={{ width: 800, height: 600 }}>
        <VirtualGrid
          items={[sampleCard]}
          selectedId={1}
          selectedIds={new Set([1])}
          columnCount={2}
          stampConfig={EMPTY_STAMP_CONFIG}
          stampArmed={false}
          stampMatchedIds={new Set()}
          onSelect={vi.fn()}
          onOpenFullView={vi.fn()}
        />
      </div>,
    );
    expect(scrollToIndex).toHaveBeenCalled();
  });

  it("skips scroll when selection is missing or unchanged", () => {
    renderGrid({ selectedId: 99 });
    expect(scrollToIndex).not.toHaveBeenCalled();
    const { rerender } = renderGrid({
      selectedId: 1,
      selectedIds: new Set([1]),
    });
    scrollToIndex.mockClear();
    rerender(
      <div style={{ width: 800, height: 600 }}>
        <VirtualGrid
          items={[sampleCard]}
          selectedId={1}
          selectedIds={new Set([1])}
          columnCount={1}
          stampConfig={EMPTY_STAMP_CONFIG}
          stampArmed={false}
          stampMatchedIds={new Set()}
          onSelect={vi.fn()}
          onOpenFullView={vi.fn()}
        />
      </div>,
    );
    expect(scrollToIndex).not.toHaveBeenCalled();
  });

  it("skips scroll repeat when strict mode re-runs unchanged selection effect", () => {
    scrollToIndex.mockClear();
    render(
      <StrictMode>
        <div style={{ width: 800, height: 600 }}>
          <VirtualGrid
            items={[sampleCard]}
            selectedId={1}
            selectedIds={new Set([1])}
            columnCount={1}
            stampConfig={EMPTY_STAMP_CONFIG}
            stampArmed={false}
            stampMatchedIds={new Set()}
            onSelect={vi.fn()}
            onOpenFullView={vi.fn()}
          />
        </div>
      </StrictMode>,
    );
    expect(scrollToIndex).toHaveBeenCalledTimes(1);
  });

  it("does not load more without handler", () => {
    const { container } = renderGrid({ hasMore: true });
    const scrollArea = container.querySelector(".grid-canvas") as HTMLElement;
    Object.defineProperty(scrollArea, "scrollHeight", {
      value: 2000,
      configurable: true,
    });
    Object.defineProperty(scrollArea, "clientHeight", {
      value: 600,
      configurable: true,
    });
    Object.defineProperty(scrollArea, "scrollTop", {
      value: 1400,
      writable: true,
      configurable: true,
    });
    fireEvent.scroll(scrollArea);
  });
});
