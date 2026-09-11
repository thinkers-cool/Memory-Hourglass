export function toggleSelection(
  selected: Set<number>,
  id: number,
  multi: boolean,
): Set<number> {
  if (!multi) {
    if (selected.has(id) && selected.size === 1) return new Set();
    return new Set([id]);
  }
  const next = new Set(selected);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  return next;
}

export function selectRange(
  items: { id: number }[],
  fromIndex: number,
  toIndex: number,
): number[] {
  const start = Math.min(fromIndex, toIndex);
  const end = Math.max(fromIndex, toIndex);
  return items.slice(start, end + 1).map((item) => item.id);
}

export function resolveSelectionAnchor(
  items: { id: number }[],
  selectedId: number | null,
  lastIndex: number | null,
): number | null {
  if (
    lastIndex !== null &&
    lastIndex >= 0 &&
    lastIndex < items.length
  ) {
    return lastIndex;
  }
  if (selectedId !== null) {
    const index = items.findIndex((item) => item.id === selectedId);
    if (index >= 0) return index;
  }
  return null;
}

export function resolveSelectionRating(
  items: { id: number; rating: number | null }[],
  selectedIds: Set<number>,
): number | null {
  if (selectedIds.size === 0) {
    return null;
  }

  const ratings = items
    .filter((item) => selectedIds.has(item.id))
    .map((item) => item.rating ?? 0);

  if (ratings.length === 0) {
    return null;
  }

  const firstRating = ratings[0];
  if (!ratings.every((rating) => rating === firstRating)) {
    return null;
  }

  return firstRating > 0 ? firstRating : null;
}

export function applyRangeSelection(
  items: { id: number }[],
  anchorIndex: number,
  targetIndex: number,
  selectedIds: Set<number>,
  additive: boolean,
): Set<number> {
  const rangeIds = selectRange(items, anchorIndex, targetIndex);
  if (additive) {
    return new Set([...selectedIds, ...rangeIds]);
  }
  return new Set(rangeIds);
}
