import type { LibraryActions } from "./libraryActions";

export function handleSelectionShortcuts(
  event: KeyboardEvent,
  actions: LibraryActions,
  options: {
    trashMode: boolean;
    purgeEnabled: boolean;
    selectionExportIds: number[];
  },
): boolean {
  if (event.key >= "1" && event.key <= "5") {
    event.preventDefault();
    actions.batchRate(Number(event.key));
    return true;
  }
  if (event.key === "t" || event.key === "T") {
    event.preventDefault();
    actions.openTagMenu();
    return true;
  }
  if (event.key === "a" || event.key === "A") {
    event.preventDefault();
    actions.openAlbumMenu();
    return true;
  }
  if (
    (event.key === "e" || event.key === "E") &&
    !event.metaKey &&
    !event.ctrlKey
  ) {
    event.preventDefault();
    if (options.selectionExportIds.length > 0) {
      actions.openExport(options.selectionExportIds);
    }
    return true;
  }
  if (event.key === "c" || event.key === "C") {
    event.preventDefault();
    actions.openCompare();
    return true;
  }
  if (options.trashMode) {
    if (
      (event.key === "r" || event.key === "R") &&
      !event.metaKey &&
      !event.ctrlKey
    ) {
      event.preventDefault();
      actions.restoreSelected();
      return true;
    }
    if (
      options.purgeEnabled &&
      event.ctrlKey &&
      (event.key === "p" || event.key === "P")
    ) {
      event.preventDefault();
      actions.batchPurge();
      return true;
    }
  } else if (event.ctrlKey && (event.key === "r" || event.key === "R")) {
    event.preventDefault();
    actions.batchRemove();
    return true;
  }
  return false;
}
