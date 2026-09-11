import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { mockLibraryActions } from "../test/fixtures";
import { useKeyboard } from "./useKeyboard";

function press(key: string, init: KeyboardEventInit = {}) {
  window.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, ...init }));
}

function renderUseKeyboard(
  actions: ReturnType<typeof mockLibraryActions>,
  options: {
    enabled?: boolean;
    fullView?: boolean;
    selectionActive?: boolean;
    trashMode?: boolean;
    purgeEnabled?: boolean;
    stampArmed?: boolean;
    selectionExportIds?: number[];
    gridExportIds?: number[];
  } = {},
) {
  return renderHook(() =>
    useKeyboard(
      actions,
      options.enabled ?? true,
      options.fullView ?? false,
      options.selectionActive ?? false,
      options.trashMode ?? false,
      options.purgeEnabled ?? true,
      options.stampArmed ?? false,
      options.selectionExportIds ?? [],
      options.gridExportIds ?? [],
    ),
  );
}

describe("useKeyboard", () => {
  it("does nothing when disabled", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions, { enabled: false });
    press("Enter");
    expect(actions.openFullView).not.toHaveBeenCalled();
  });

  it("opens full view on Enter in grid mode", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions);
    press("Enter");
    expect(actions.openFullView).toHaveBeenCalledTimes(1);
  });

  it("batch rates selected items", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions, { selectionActive: true, selectionExportIds: [1], gridExportIds: [1] });
    press("3");
    expect(actions.batchRate).toHaveBeenCalledWith(3);
  });

  it("restores items in trash mode", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions, {
      selectionActive: true,
      trashMode: true,
      selectionExportIds: [1],
      gridExportIds: [1],
    });
    press("r");
    expect(actions.restoreSelected).toHaveBeenCalledTimes(1);
  });

  it("closes full view on Escape", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions, { fullView: true });
    press("Escape");
    expect(actions.closeFullView).toHaveBeenCalledTimes(1);
  });

  it("ignores shortcuts while typing in inputs", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions);
    const input = document.createElement("input");
    document.body.appendChild(input);
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    expect(actions.openFullView).not.toHaveBeenCalled();
    input.remove();
  });

  it("stamps on space when armed", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions, { stampArmed: true });
    press(" ");
    expect(actions.toggleStampOnTargets).toHaveBeenCalledTimes(1);
    expect(actions.openGallery).not.toHaveBeenCalled();
  });

  it("opens gallery on F12", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions);
    press("F12");
    expect(actions.openGallery).toHaveBeenCalledTimes(1);
  });

  it("stamps in full view on space when armed", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions, { fullView: true, stampArmed: true });
    press(" ");
    expect(actions.toggleStampOnTargets).toHaveBeenCalledTimes(1);
  });

  it("adjusts grid size with plus and minus", () => {
    const actions = mockLibraryActions();
    renderUseKeyboard(actions);
    press("-");
    press("=");
    expect(actions.adjustGridSize).toHaveBeenCalledWith(-1);
    expect(actions.adjustGridSize).toHaveBeenCalledWith(1);
  });

  describe("full view", () => {
    it("rates with keys 1-5", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, { fullView: true });
      press("4");
      expect(actions.rate).toHaveBeenCalledWith(4);
    });

    it("navigates relative with arrow keys", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, { fullView: true });
      press("ArrowLeft");
      press("ArrowRight");
      expect(actions.navigateRelative).toHaveBeenCalledWith(-1);
      expect(actions.navigateRelative).toHaveBeenCalledWith(1);
    });

    it("handles selection shortcuts in full view", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, {
        fullView: true,
        selectionActive: true,
        selectionExportIds: [1],
        gridExportIds: [1],
      });
      press("t");
      expect(actions.openTagMenu).toHaveBeenCalledTimes(1);
      press("a");
      expect(actions.openAlbumMenu).toHaveBeenCalledTimes(1);
      press("e");
      expect(actions.openExport).toHaveBeenCalledWith([1]);
      press("c");
      expect(actions.openCompare).toHaveBeenCalledTimes(1);
    });

    it("ignores unmatched keys in full view", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, { fullView: true });
      press("x");
      expect(actions.closeFullView).not.toHaveBeenCalled();
      expect(actions.navigateRelative).not.toHaveBeenCalled();
    });

    it("ignores shortcuts while typing in full view inputs", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, { fullView: true });
      const input = document.createElement("textarea");
      document.body.appendChild(input);
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
      expect(actions.closeFullView).not.toHaveBeenCalled();
      input.remove();
    });
  });

  describe("grid", () => {
    it("toggles fullscreen with g/G", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions);
      press("g");
      press("G");
      expect(actions.toggleFullscreen).toHaveBeenCalledTimes(2);
    });

    it("dispatches focus-filter on f/F", () => {
      const actions = mockLibraryActions();
      const handler = vi.fn();
      window.addEventListener("memhg:focus-filter", handler);
      renderUseKeyboard(actions);
      press("f");
      expect(handler).toHaveBeenCalledTimes(1);
      window.removeEventListener("memhg:focus-filter", handler);
    });

    it("opens grid export on e/E when ids exist", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, { gridExportIds: [1, 2] });
      press("e");
      expect(actions.openExport).toHaveBeenCalledWith([1, 2]);
    });

    it("skips grid export when no ids", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions);
      press("e");
      expect(actions.openExport).not.toHaveBeenCalled();
    });

    it("navigates relative without shift on horizontal arrows", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions);
      press("ArrowLeft");
      press("ArrowRight");
      expect(actions.navigateRelative).toHaveBeenCalledWith(-1);
      expect(actions.navigateRelative).toHaveBeenCalledWith(1);
      expect(actions.navigateGrid).not.toHaveBeenCalled();
    });

    it("navigates grid with shift on horizontal arrows", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions);
      press("ArrowLeft", { shiftKey: true });
      press("ArrowRight", { shiftKey: true });
      expect(actions.navigateGrid).toHaveBeenCalledWith(-1, 0);
      expect(actions.navigateGrid).toHaveBeenCalledWith(1, 0);
    });

    it("ignores escape in grid mode", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions);
      press("Escape");
      expect(actions.closeFullView).not.toHaveBeenCalled();
      expect(actions.openFullView).not.toHaveBeenCalled();
    });

    it("navigates grid vertically", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions);
      press("ArrowUp");
      press("ArrowDown");
      expect(actions.navigateGrid).toHaveBeenCalledWith(0, -1);
      expect(actions.navigateGrid).toHaveBeenCalledWith(0, 1);
    });
  });

  describe("selection shortcuts", () => {
    it("rates without selection using 1-5", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions);
      press("2");
      expect(actions.rate).toHaveBeenCalledWith(2);
    });

    it("opens tag, album, export, and compare menus", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, { selectionActive: true, selectionExportIds: [5] });
      press("T");
      press("A");
      press("E");
      press("C");
      expect(actions.openTagMenu).toHaveBeenCalledTimes(1);
      expect(actions.openAlbumMenu).toHaveBeenCalledTimes(1);
      expect(actions.openExport).toHaveBeenCalledWith([5]);
      expect(actions.openCompare).toHaveBeenCalledTimes(1);
    });

    it("batch purges in trash mode with ctrl+P", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, {
        selectionActive: true,
        trashMode: true,
        selectionExportIds: [1],
      });
      press("p", { ctrlKey: true });
      expect(actions.batchPurge).toHaveBeenCalledTimes(1);
    });

    it("skips batch purge when purge is disabled", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, {
        selectionActive: true,
        trashMode: true,
        purgeEnabled: false,
        selectionExportIds: [1],
      });
      press("p", { ctrlKey: true });
      expect(actions.batchPurge).not.toHaveBeenCalled();
    });

    it("batch removes with ctrl+R outside trash", () => {
      const actions = mockLibraryActions();
      renderUseKeyboard(actions, { selectionActive: true, selectionExportIds: [1] });
      press("r", { ctrlKey: true });
      expect(actions.batchRemove).toHaveBeenCalledTimes(1);
    });
  });
});
