import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createLocalStorageMock } from "../test/storageMock";
import {
  INSPECTOR_WIDTH_DEFAULT,
  INSPECTOR_WIDTH_MAX,
  INSPECTOR_WIDTH_MIN,
  INSPECTOR_STORAGE_KEY,
  LEFT_PANEL_WIDTH_DEFAULT,
  LEFT_PANEL_WIDTH_MAX,
  LEFT_PANEL_WIDTH_MIN,
  LEFT_PANEL_STORAGE_KEY,
  clampPanelWidth,
  loadPanelWidth,
  savePanelWidth,
} from "./panelLayout";

describe("clampPanelWidth", () => {
  it("rounds and clamps within bounds", () => {
    expect(clampPanelWidth(250.6, LEFT_PANEL_WIDTH_MIN, LEFT_PANEL_WIDTH_MAX)).toBe(
      251,
    );
    expect(clampPanelWidth(50, LEFT_PANEL_WIDTH_MIN, LEFT_PANEL_WIDTH_MAX)).toBe(
      LEFT_PANEL_WIDTH_MIN,
    );
    expect(clampPanelWidth(999, LEFT_PANEL_WIDTH_MIN, LEFT_PANEL_WIDTH_MAX)).toBe(
      LEFT_PANEL_WIDTH_MAX,
    );
  });
});

describe("loadPanelWidth", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", createLocalStorageMock());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns fallback when storage is empty", () => {
    expect(
      loadPanelWidth(
        LEFT_PANEL_STORAGE_KEY,
        LEFT_PANEL_WIDTH_DEFAULT,
        LEFT_PANEL_WIDTH_MIN,
        LEFT_PANEL_WIDTH_MAX,
      ),
    ).toBe(LEFT_PANEL_WIDTH_DEFAULT);
  });

  it("loads and clamps stored width", () => {
    localStorage.setItem(LEFT_PANEL_STORAGE_KEY, "400");
    expect(
      loadPanelWidth(
        LEFT_PANEL_STORAGE_KEY,
        LEFT_PANEL_WIDTH_DEFAULT,
        LEFT_PANEL_WIDTH_MIN,
        LEFT_PANEL_WIDTH_MAX,
      ),
    ).toBe(400);
  });

  it("returns fallback for invalid stored values", () => {
    localStorage.setItem(INSPECTOR_STORAGE_KEY, "not-a-number");
    expect(
      loadPanelWidth(
        INSPECTOR_STORAGE_KEY,
        INSPECTOR_WIDTH_DEFAULT,
        INSPECTOR_WIDTH_MIN,
        INSPECTOR_WIDTH_MAX,
      ),
    ).toBe(INSPECTOR_WIDTH_DEFAULT);
  });

  it("returns fallback when storage throws", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    });
    expect(
      loadPanelWidth(
        LEFT_PANEL_STORAGE_KEY,
        LEFT_PANEL_WIDTH_DEFAULT,
        LEFT_PANEL_WIDTH_MIN,
        LEFT_PANEL_WIDTH_MAX,
      ),
    ).toBe(LEFT_PANEL_WIDTH_DEFAULT);
  });
});

describe("savePanelWidth", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", createLocalStorageMock());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("persists width as string", () => {
    savePanelWidth(LEFT_PANEL_STORAGE_KEY, 260);
    expect(localStorage.getItem(LEFT_PANEL_STORAGE_KEY)).toBe("260");
  });

  it("ignores storage write failures", () => {
    vi.stubGlobal("localStorage", {
      setItem: () => {
        throw new Error("blocked");
      },
    });
    expect(() => savePanelWidth(LEFT_PANEL_STORAGE_KEY, 260)).not.toThrow();
  });
});
