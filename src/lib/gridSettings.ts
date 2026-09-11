import { loadStoredNumber, saveStoredNumber } from "./storage";

export const GRID_COLUMN_COUNT_MIN = 2;
export const GRID_COLUMN_COUNT_MAX = 16;
export const GRID_COLUMN_COUNT_DEFAULT = 5;
export const GRID_COLUMN_COUNT_STEP = 1;
export const GRID_GAP_PX = 8;
export const GRID_COLUMN_STORAGE_KEY = "memhg.gridColumnCount";

export function clampGridColumnCount(count: number): number {
  return Math.max(
    GRID_COLUMN_COUNT_MIN,
    Math.min(GRID_COLUMN_COUNT_MAX, Math.round(count)),
  );
}

export function loadGridColumnCount(): number {
  return loadStoredNumber(
    GRID_COLUMN_STORAGE_KEY,
    GRID_COLUMN_COUNT_DEFAULT,
    clampGridColumnCount,
  );
}

export function saveGridColumnCount(count: number): void {
  saveStoredNumber(GRID_COLUMN_STORAGE_KEY, clampGridColumnCount(count));
}

export function cellWidthForColumn(
  width: number,
  columnCount: number,
  gap = GRID_GAP_PX,
): number {
  if (columnCount < 1) return width;
  return (width - gap * (columnCount - 1)) / columnCount;
}
