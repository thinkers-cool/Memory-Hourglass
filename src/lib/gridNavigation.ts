export function gridIndex(
  row: number,
  col: number,
  columnCount: number,
): number {
  return row * columnCount + col;
}

export function gridPosition(
  index: number,
  columnCount: number,
): { row: number; col: number } {
  return {
    row: Math.floor(index / columnCount),
    col: index % columnCount,
  };
}

export function navigateGridIndex(
  currentIndex: number,
  deltaCol: number,
  deltaRow: number,
  columnCount: number,
  itemCount: number,
): number {
  if (itemCount === 0 || columnCount < 1) return 0;
  if (currentIndex < 0) return 0;

  const { row, col } = gridPosition(currentIndex, columnCount);
  const targetRow = row + deltaRow;
  const targetCol = col + deltaCol;

  if (targetRow < 0 || targetCol < 0) return currentIndex;

  const lastRow = Math.floor((itemCount - 1) / columnCount);
  if (targetRow > lastRow) return currentIndex;

  const itemsInTargetRow = Math.min(
    columnCount,
    itemCount - targetRow * columnCount,
  );
  if (targetCol >= itemsInTargetRow) return currentIndex;

  const nextIndex = gridIndex(targetRow, targetCol, columnCount);
  return Math.min(nextIndex, itemCount - 1);
}
