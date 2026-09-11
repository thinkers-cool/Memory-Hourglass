import { memo, useCallback } from "react";
import { AssetGridCard } from "./AssetGridCard";
import type { AssetCard } from "../types";
import type { StampIndicatorState } from "../lib/stamp";

type VirtualGridRowProps = {
  virtualRowIndex: number;
  virtualRowStart: number;
  cellSizePx: number;
  columnCount: number;
  gapPx: number;
  items: AssetCard[];
  selectedId: number | null;
  selectedIds: Set<number>;
  stampIndicatorFor: (card: AssetCard) => StampIndicatorState;
  onSelect: (card: AssetCard, multi: boolean, range: boolean) => void;
  onOpenFullView: (card: AssetCard) => void;
};

function VirtualGridRowInner({
  virtualRowIndex,
  virtualRowStart,
  cellSizePx,
  columnCount,
  gapPx,
  items,
  selectedId,
  selectedIds,
  stampIndicatorFor,
  onSelect,
  onOpenFullView,
}: VirtualGridRowProps) {
  const handleSelect = useCallback(
    (card: AssetCard, multi: boolean, range: boolean) => {
      onSelect(card, multi, range);
    },
    [onSelect],
  );

  const handleOpen = useCallback(
    (card: AssetCard) => {
      onOpenFullView(card);
    },
    [onOpenFullView],
  );

  return (
    <div
      className="absolute top-0 left-0 grid w-full items-start"
      style={{
        height: `${cellSizePx}px`,
        transform: `translateY(${virtualRowStart}px)`,
        gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
        gap: `${gapPx}px`,
      }}
    >
      {Array.from({ length: columnCount }, (_, col) => {
        const card = items[virtualRowIndex * columnCount + col];
        if (!card) return <div key={col} />;
        return (
          <AssetGridCard
            key={`${virtualRowIndex}-${card.id}`}
            card={card}
            selected={selectedId === card.id || selectedIds.has(card.id)}
            stampIndicator={stampIndicatorFor(card)}
            onSelect={(multi, range) => handleSelect(card, multi, range)}
            onOpenFullView={() => handleOpen(card)}
          />
        );
      })}
    </div>
  );
}

export const VirtualGridRow = memo(VirtualGridRowInner);
