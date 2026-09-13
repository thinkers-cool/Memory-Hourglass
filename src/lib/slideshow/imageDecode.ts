import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { AssetCard } from "../../types";

const decodeCache = new Map<string, Promise<void>>();
const decodedSrcs = new Set<string>();

export function clearDecodeCache(): void {
  decodeCache.clear();
  decodedSrcs.clear();
}

export function isImageDecoded(src: string): boolean {
  return decodedSrcs.has(src);
}

export function mediaSrc(card: AssetCard): string {
  return convertFileSrc(card.abs_path);
}

export function decodeImage(src: string): Promise<void> {
  if (decodedSrcs.has(src)) return Promise.resolve();

  const cached = decodeCache.get(src);
  if (cached) return cached;

  const pending = new Promise<void>((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const finish = () => {
        decodedSrcs.add(src);
        resolve();
      };
      if (img.decode) {
        void img.decode().then(finish).catch(finish);
      } else {
        finish();
      }
    };
    img.onerror = () => reject(new Error("image decode failed"));
    img.src = src;
  }).catch((error: unknown) => {
    decodeCache.delete(src);
    throw error;
  });

  decodeCache.set(src, pending);
  return pending;
}

export function useDecodedImage(src: string | null, immediate = false): boolean {
  const [ready, setReady] = useState(
    () => immediate || !src || isImageDecoded(src),
  );

  useEffect(() => {
    if (immediate || !src) {
      setReady(true);
      return;
    }
    if (isImageDecoded(src)) {
      setReady(true);
      return;
    }
    let cancelled = false;
    setReady(false);
    void decodeImage(src)
      .then(() => {
        if (!cancelled) setReady(true);
      })
      .catch(() => {
        if (!cancelled) setReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, [src, immediate]);

  return ready;
}

export function useSlideImageReady(card: AssetCard | undefined): boolean {
  const isVideo = card?.kind === "video";
  const src = card && !isVideo ? mediaSrc(card) : null;
  return useDecodedImage(src, isVideo || !card);
}
