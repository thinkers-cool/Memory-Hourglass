import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect } from "react";
import type { AssetCard } from "../../types";
import { isPlayable } from "./navigation";
import { indicesInWindow } from "./preloadWindow";

const PRELOAD_WINDOW = 1;
const PRELOAD_TIMEOUT_MS = 3000;

function mediaSrc(card: AssetCard): string {
  return convertFileSrc(card.abs_path);
}

function thumbSrc(card: AssetCard): string | null {
  return card.thumb_path ? convertFileSrc(card.thumb_path) : null;
}

function preloadImage(src: string): void {
  const img = new Image();
  img.src = src;
  if (img.decode) {
    void img.decode().catch(() => undefined);
  }
}

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
      const thumb = thumbSrc(card);
      if (thumb) preloadImage(thumb);
      if (card.kind === "video") {
        preloadVideoMeta(mediaSrc(card));
      } else {
        preloadImage(mediaSrc(card));
      }
    }
  }, [items, index]);
}
