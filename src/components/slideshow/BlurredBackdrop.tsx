import { convertFileSrc } from "@tauri-apps/api/core";
import type { AssetCard } from "../../types";

export function BlurredBackdrop({
  card,
  opacity = 0.35,
}: {
  card: AssetCard;
  opacity?: number;
}) {
  const src = card.thumb_path
    ? convertFileSrc(card.thumb_path)
    : convertFileSrc(card.abs_path);

  return (
    <img
      src={src}
      alt=""
      aria-hidden
      className="pointer-events-none absolute inset-0 h-full w-full scale-110 object-cover blur-3xl"
      style={{ opacity }}
      draggable={false}
    />
  );
}
