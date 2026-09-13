import { useEffect } from "react";
import type { AssetCard } from "../../types";
import { decodeImage, mediaSrc } from "./imageDecode";
import { isPlayable } from "./navigation";
import { indicesInWindow } from "./preloadWindow";

const PRELOAD_WINDOW = 2;
const PRELOAD_TIMEOUT_MS = 3000;

function preloadVideoMeta(src: string): void {
  const video = document.createElement("video");
  video.preload = "metadata";
  video.src = src;
  const timer = window.setTimeout(() => {
    video.removeAttribute("src");
    video.load();
  }, PRELOAD_TIMEOUT_MS);
  video.onloadedmetadata = () => window.clearTimeout(timer);
  video.onerror = () => window.clearTimeout(timer);
}

export function useSlideshowPreload(items: AssetCard[], index: number): void {
  useEffect(() => {
    for (const i of indicesInWindow(index, items.length, PRELOAD_WINDOW)) {
      const card = items[i];
      if (!card || !isPlayable(card)) continue;
      if (card.kind === "video") {
        preloadVideoMeta(mediaSrc(card));
      } else {
        void decodeImage(mediaSrc(card)).catch(() => undefined);
      }
    }
  }, [items, index]);
}
