import type { AssetCard, ScanThumbUpdate } from "../types";

export function patchAssetThumbs(
  items: AssetCard[],
  thumbs: ScanThumbUpdate[],
): AssetCard[] {
  if (thumbs.length === 0) {
    return items;
  }
  const thumbById = new Map(thumbs.map((thumb) => [thumb.asset_id, thumb.thumb_path]));
  let changed = false;
  const next = items.map((item) => {
    const thumbPath = thumbById.get(item.id);
    if (thumbPath && thumbPath !== item.thumb_path) {
      changed = true;
      return { ...item, thumb_path: thumbPath };
    }
    return item;
  });
  return changed ? next : items;
}
