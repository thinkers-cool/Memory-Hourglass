import { describe, expect, it, vi } from "vitest";
import { mockLibraryActions } from "../test/fixtures";
import { handleSelectionShortcuts } from "./keyboardShortcuts";

function keyEvent(key: string, init: KeyboardEventInit = {}) {
  return new KeyboardEvent("keydown", { key, bubbles: true, ...init });
}

const libraryOptions = {
  trashMode: false,
  purgeEnabled: true,
  selectionExportIds: [] as number[],
};

describe("handleSelectionShortcuts", () => {
  it("handles rating, tag, album, export, compare", () => {
    const actions = mockLibraryActions();
    expect(
      handleSelectionShortcuts(keyEvent("3"), actions, {
        ...libraryOptions,
        selectionExportIds: [1],
      }),
    ).toBe(true);
    expect(actions.batchRate).toHaveBeenCalledWith(3);

    handleSelectionShortcuts(keyEvent("t"), actions, libraryOptions);
    handleSelectionShortcuts(keyEvent("A"), actions, libraryOptions);
    handleSelectionShortcuts(keyEvent("e"), actions, {
      ...libraryOptions,
      selectionExportIds: [1, 2],
    });
    handleSelectionShortcuts(keyEvent("c"), actions, libraryOptions);

    expect(actions.openTagMenu).toHaveBeenCalled();
    expect(actions.openAlbumMenu).toHaveBeenCalled();
    expect(actions.openExport).toHaveBeenCalledWith([1, 2]);
    expect(actions.openCompare).toHaveBeenCalled();
  });

  it("skips export when ctrl/meta held or no ids", () => {
    const actions = mockLibraryActions();
    handleSelectionShortcuts(keyEvent("e", { ctrlKey: true }), actions, {
      ...libraryOptions,
      selectionExportIds: [1],
    });
    handleSelectionShortcuts(keyEvent("E", { metaKey: true }), actions, {
      ...libraryOptions,
      selectionExportIds: [1],
    });
    handleSelectionShortcuts(keyEvent("e"), actions, libraryOptions);
    expect(actions.openExport).not.toHaveBeenCalled();
  });

  it("handles trash restore and purge", () => {
    const actions = mockLibraryActions();
    handleSelectionShortcuts(keyEvent("r"), actions, {
      ...libraryOptions,
      trashMode: true,
    });
    handleSelectionShortcuts(keyEvent("p", { ctrlKey: true }), actions, {
      ...libraryOptions,
      trashMode: true,
    });
    expect(actions.restoreSelected).toHaveBeenCalled();
    expect(actions.batchPurge).toHaveBeenCalled();
  });

  it("skips purge shortcut when purge is disabled", () => {
    const actions = mockLibraryActions();
    handleSelectionShortcuts(keyEvent("p", { ctrlKey: true }), actions, {
      ...libraryOptions,
      trashMode: true,
      purgeEnabled: false,
    });
    expect(actions.batchPurge).not.toHaveBeenCalled();
  });

  it("handles library batch remove", () => {
    const actions = mockLibraryActions();
    handleSelectionShortcuts(keyEvent("r", { ctrlKey: true }), actions, libraryOptions);
    expect(actions.batchRemove).toHaveBeenCalled();
  });

  it("returns false for unrelated keys", () => {
    const actions = mockLibraryActions();
    expect(handleSelectionShortcuts(keyEvent("z"), actions, libraryOptions)).toBe(false);
  });

  it("handles uppercase remove and purge shortcuts", () => {
    const actions = mockLibraryActions();
    handleSelectionShortcuts(keyEvent("R", { ctrlKey: true }), actions, libraryOptions);
    handleSelectionShortcuts(keyEvent("P", { ctrlKey: true }), actions, {
      ...libraryOptions,
      trashMode: true,
    });
    expect(actions.batchRemove).toHaveBeenCalled();
    expect(actions.batchPurge).toHaveBeenCalled();
  });
});
