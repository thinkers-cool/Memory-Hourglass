import { useVirtualizer } from "@tanstack/react-virtual";
import { useCallback, useEffect, useRef, useState } from "react";
import type { AssetCard } from "../types";
import { cellWidthForColumn, GRID_GAP_PX } from "../lib/gridSettings";
import { stampIndicatorState, type StampConfig } from "../lib/stamp";
import { VirtualGridRow } from "./VirtualGridRow";

export function VirtualGrid({
  items,
  selectedId,
  selectedIds,
  columnCount,
  stampConfig,
  stampArmed,
  stampMatchedIds,
  onSelect,
  onOpenFullView,
  onLoadMore,
  hasMore,
  loadingMore,
}: {
  items: AssetCard[];
  selectedId: number | null;
  selectedIds: Set<number>;
  columnCount: number;
  stampConfig: StampConfig;
  stampArmed: boolean;
  stampMatchedIds: ReadonlySet<number>;
  onSelect: (card: AssetCard, multi: boolean, range: boolean) => void;
  onOpenFullView: (card: AssetCard) => void;
  onLoadMore?: () => void;
  hasMore?: boolean;
  loadingMore?: boolean;
}) {
  const parentRef = useRef<HTMLDivElement>(null);
  const itemsRef = useRef(items);
  const columnCountRef = useRef(columnCount);
  const prevSelectedIdRef = useRef<number | null>(null);
  const prevScrollColumnCountRef = useRef(columnCount);
  const [cellSizePx, setCellSizePx] = useState(140);

  itemsRef.current = items;
  columnCountRef.current = columnCount;

  const applyLayout = useCallback((width: number, columns: number) => {
    setCellSizePx(cellWidthForColumn(width, columns));
  }, []);

  useEffect(() => {
    const el = parentRef.current!;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? 800;
      applyLayout(width, columnCountRef.current);
    });

    observer.observe(el);
    applyLayout(el.clientWidth, columnCount);

    return () => observer.disconnect();
  }, [applyLayout, columnCount]);

  useEffect(() => {
    const el = parentRef.current!;
    applyLayout(el.clientWidth, columnCount);
  }, [columnCount, applyLayout]);

  const rowCount = Math.ceil(items.length / columnCount);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => cellSizePx,
    gap: GRID_GAP_PX,
    overscan: 3,
  });

  useEffect(() => {
    rowVirtualizer.measure();
  }, [cellSizePx, columnCount, rowVirtualizer]);

  const stampIndicatorFor = useCallback(
    (card: AssetCard) =>
      stampIndicatorState(card, stampConfig, stampArmed, stampMatchedIds),
    [stampArmed, stampConfig, stampMatchedIds],
  );

  const scrollToSelectedRowRef = useRef<() => void>(null!);
  scrollToSelectedRowRef.current = () => {
    const idx = itemsRef.current.findIndex((item) => item.id === selectedId);
    if (idx < 0) return;
    const row = Math.floor(idx / columnCountRef.current);
    rowVirtualizer.scrollToIndex(row, { align: "auto" });
  };

  useEffect(() => {
    if (selectedId === null) {
      prevSelectedIdRef.current = null;
      return;
    }

    const selectionChanged = selectedId !== prevSelectedIdRef.current;
    const columnsChanged = columnCount !== prevScrollColumnCountRef.current;
    if (!selectionChanged && !columnsChanged) return;

    prevSelectedIdRef.current = selectedId;
    prevScrollColumnCountRef.current = columnCount;
    scrollToSelectedRowRef.current();
  }, [selectedId, columnCount]);

  useEffect(() => {
    const el = parentRef.current!;
    const onScroll = () => {
      if (!onLoadMore || !hasMore || loadingMore) return;
      const nearBottom =
        el.scrollHeight - el.scrollTop - el.clientHeight <
        (cellSizePx + GRID_GAP_PX) * 2;
      if (nearBottom) onLoadMore();
    };
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [onLoadMore, hasMore, loadingMore, cellSizePx]);

  const selectionPadding = selectedIds.size > 0 ? "pb-24" : "";

  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      <div
        ref={parentRef}
        className={`grid-canvas min-h-0 flex-1 overflow-y-auto p-3 ${selectionPadding}`}
      >
        <div
          className="relative w-full"
          style={{ height: `${rowVirtualizer.getTotalSize()}px` }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualRow) => (
            <VirtualGridRow
              key={virtualRow.key}
              virtualRowIndex={virtualRow.index}
              virtualRowStart={virtualRow.start}
              cellSizePx={cellSizePx}
              columnCount={columnCount}
              gapPx={GRID_GAP_PX}
              items={items}
              selectedId={selectedId}
              selectedIds={selectedIds}
              stampIndicatorFor={stampIndicatorFor}
              onSelect={onSelect}
              onOpenFullView={onOpenFullView}
            />
          ))}
        </div>
      </div>
      {loadingMore && (
        <div className="pointer-events-none absolute bottom-4 left-1/2 -translate-x-1/2">
          <span className="loading loading-spinner loading-sm opacity-50" />
        </div>
      )}
    </div>
  );
}
