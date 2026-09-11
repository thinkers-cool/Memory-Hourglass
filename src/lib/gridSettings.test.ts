import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createLocalStorageMock } from "../test/storageMock";
import {
  GRID_COLUMN_COUNT_DEFAULT,
  GRID_COLUMN_COUNT_MAX,
  GRID_COLUMN_COUNT_MIN,
  GRID_COLUMN_STORAGE_KEY,
  GRID_GAP_PX,
  cellWidthForColumn,
  clampGridColumnCount,
  loadGridColumnCount,
  saveGridColumnCount,
} from "./gridSettings";

describe("gridSettings", () => {
  it("clamps column count", () => {
    expect(clampGridColumnCount(1)).toBe(GRID_COLUMN_COUNT_MIN);
    expect(clampGridColumnCount(20)).toBe(GRID_COLUMN_COUNT_MAX);
    expect(clampGridColumnCount(7)).toBe(7);
  });

  it("computes cell width for column layout", () => {
    expect(cellWidthForColumn(800, 5, GRID_GAP_PX)).toBe(153.6);
    expect(cellWidthForColumn(800, 0, GRID_GAP_PX)).toBe(800);
  });

  describe("localStorage persistence", () => {
    beforeEach(() => {
      vi.stubGlobal("localStorage", createLocalStorageMock());
    });

    afterEach(() => {
      vi.unstubAllGlobals();
    });

    it("loads clamped column count from storage", () => {
      localStorage.setItem(GRID_COLUMN_STORAGE_KEY, "8");
      expect(loadGridColumnCount()).toBe(8);
    });

    it("returns default for invalid stored values", () => {
      localStorage.setItem(GRID_COLUMN_STORAGE_KEY, "invalid");
      expect(loadGridColumnCount()).toBe(GRID_COLUMN_COUNT_DEFAULT);
    });

    it("returns default when storage throws on read", () => {
      vi.stubGlobal("localStorage", {
        getItem: () => {
          throw new Error("blocked");
        },
      });
      expect(loadGridColumnCount()).toBe(GRID_COLUMN_COUNT_DEFAULT);
    });

    it("saves clamped column count", () => {
      saveGridColumnCount(99);
      expect(localStorage.getItem(GRID_COLUMN_STORAGE_KEY)).toBe(
        String(GRID_COLUMN_COUNT_MAX),
      );
    });

    it("ignores storage write failures", () => {
      vi.stubGlobal("localStorage", {
        setItem: () => {
          throw new Error("blocked");
        },
      });
      expect(() => saveGridColumnCount(5)).not.toThrow();
    });
  });
});
