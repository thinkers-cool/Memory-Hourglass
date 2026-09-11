import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { AssetCard } from "../../types";

export function BlurredBackdrop({ card }: { card: AssetCard }) {
  const src = card.thumb_path
    ? convertFileSrc(card.thumb_path)
    : convertFileSrc(card.abs_path);

  return (
    <img
      src={src}
      alt=""
      aria-hidden
      className="pointer-events-none absolute inset-0 h-full w-full scale-110 object-cover opacity-35 blur-3xl"
      draggable={false}
    />
  );
}

export function useProgressiveImage(card: AssetCard) {
  const fullSrc = convertFileSrc(card.abs_path);
  const previewSrc = card.thumb_path ? convertFileSrc(card.thumb_path) : fullSrc;
  const [src, setSrc] = useState(previewSrc);

  useEffect(() => {
    setSrc(previewSrc);
    if (previewSrc === fullSrc) return;
    const img = new Image();
    img.src = fullSrc;
    const apply = () => setSrc(fullSrc);
    img.onload = apply;
    if (img.decode) {
      void img.decode().then(apply).catch(apply);
    }
    return () => {
      img.onload = null;
    };
  }, [card.id, fullSrc, previewSrc]);

  return src;
}
