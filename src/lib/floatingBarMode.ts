import type { DeleteFilterStatus } from "./libraryFilters";

export type FloatingBarMode = "library" | "trash";

export function resolveFloatingBarMode(
  deleteStatus: DeleteFilterStatus,
): FloatingBarMode {
  if (deleteStatus === "deleted") return "trash";
  return "library";
}

export function shouldShowFloatingBar(
  selectedCount: number,
  galleryOpen: boolean,
  compareOpen: boolean,
): boolean {
  if (selectedCount === 0) return false;
  if (galleryOpen || compareOpen) return false;
  return true;
}
