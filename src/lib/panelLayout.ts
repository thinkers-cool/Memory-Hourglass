import { loadStoredNumber, saveStoredNumber } from "./storage";

export const LEFT_PANEL_WIDTH_MIN = 180;
export const LEFT_PANEL_WIDTH_MAX = 480;
export const LEFT_PANEL_WIDTH_DEFAULT = 224;
export const LEFT_PANEL_STORAGE_KEY = "memhg.leftPanelWidth";

export const INSPECTOR_WIDTH_MIN = 260;
export const INSPECTOR_WIDTH_MAX = 560;
export const INSPECTOR_WIDTH_DEFAULT = 340;
export const INSPECTOR_STORAGE_KEY = "memhg.inspectorWidth";

export function clampPanelWidth(
  width: number,
  min: number,
  max: number,
): number {
  return Math.max(min, Math.min(max, Math.round(width)));
}

export function loadPanelWidth(
  key: string,
  fallback: number,
  min: number,
  max: number,
): number {
  return loadStoredNumber(key, fallback, (value) =>
    clampPanelWidth(value, min, max),
  );
}

export function savePanelWidth(key: string, width: number): void {
  saveStoredNumber(key, width);
}
