import { useEffect } from "react";
import type { LibraryActions } from "../lib/libraryActions";
import { isTextEntryElement } from "../lib/focus";
import {
  handleSelectionShortcuts,
} from "../lib/keyboardShortcuts";

export function useKeyboard(
  actions: LibraryActions,
  enabled: boolean,
  fullView: boolean,
  selectionActive: boolean,
  trashMode: boolean,
  purgeEnabled: boolean,
  stampArmed: boolean,
  selectionExportIds: number[],
  gridExportIds: number[],
) {
  useEffect(() => {
    if (!enabled) return;

    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (!fullView && isTextEntryElement(target)) {
        return;
      }

      if (fullView) {
        if (isTextEntryElement(target)) {
          return;
        }

        if (e.key === "Escape") {
          e.preventDefault();
          actions.closeFullView();
          return;
        }
        if (e.key >= "1" && e.key <= "5") {
          e.preventDefault();
          actions.rate(Number(e.key));
          return;
        }
        if (e.key === "ArrowLeft") {
          e.preventDefault();
          actions.navigateRelative(-1);
          return;
        }
        if (e.key === "ArrowRight") {
          e.preventDefault();
          actions.navigateRelative(1);
          return;
        }
        if (e.key === " " || e.code === "Space") {
          e.preventDefault();
          if (stampArmed) {
            actions.toggleStampOnTargets();
          }
          return;
        }
        if (
          selectionActive &&
          handleSelectionShortcuts(e, actions, {
            trashMode,
            purgeEnabled,
            selectionExportIds,
          })
        ) {
          return;
        }
        return;
      }

      if (e.key === "Escape") {
        return;
      }

      if (e.key === "Enter") {
        e.preventDefault();
        actions.openFullView();
        return;
      }

      if (
        selectionActive &&
        handleSelectionShortcuts(e, actions, {
          trashMode,
          purgeEnabled,
          selectionExportIds,
        })
      ) {
        return;
      }

      if (!selectionActive && e.key >= "1" && e.key <= "5") {
        e.preventDefault();
        actions.rate(Number(e.key));
        return;
      }

      if (e.key === "g" || e.key === "G") {
        e.preventDefault();
        actions.toggleFullscreen();
        return;
      }

      if (e.key === "f" || e.key === "F") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("memhg:focus-filter"));
        return;
      }

      if (e.key === "e" || e.key === "E") {
        if (!e.metaKey && !e.ctrlKey && gridExportIds.length > 0) {
          e.preventDefault();
          actions.openExport(gridExportIds);
        }
        return;
      }

      if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
        e.preventDefault();
        const delta = e.key === "ArrowLeft" ? -1 : 1;
        if (e.shiftKey) {
          actions.navigateGrid(delta, 0);
        } else {
          actions.navigateRelative(delta);
        }
        return;
      }

      if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        e.preventDefault();
        const delta = e.key === "ArrowUp" ? -1 : 1;
        actions.navigateGrid(0, delta);
        return;
      }

      if (e.key === " " || e.code === "Space") {
        e.preventDefault();
        if (stampArmed) {
          actions.toggleStampOnTargets();
        }
        return;
      }

      if (e.key === "F12") {
        e.preventDefault();
        actions.openGallery();
        return;
      }

      if (e.key === "=" || e.key === "+") {
        e.preventDefault();
        actions.adjustGridSize(1);
        return;
      }

      if (e.key === "-") {
        e.preventDefault();
        actions.adjustGridSize(-1);
        return;
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    actions,
    enabled,
    fullView,
    gridExportIds,
    selectionActive,
    selectionExportIds,
    stampArmed,
    trashMode,
    purgeEnabled,
  ]);
}
