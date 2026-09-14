import { convertFileSrc } from "@tauri-apps/api/core";
import type { AssetCard } from "../types";

export function preloadFullViewImage(absPath: string): Promise<void> {
  const img = new Image();
  const src = convertFileSrc(absPath);
  img.src = src;
  if (img.complete) {
    return Promise.resolve();
  }
  return new Promise((resolve, reject) => {
    img.onload = () => resolve();
    img.onerror = () => reject(new Error("full view image failed to load"));
  });
}

export function prefetchFullViewNeighbors(cards: AssetCard[]): void {
  for (const card of cards) {
    if (card.kind !== "image") {
      continue;
    }
    void preloadFullViewImage(card.abs_path).catch(() => {});
  }
}
