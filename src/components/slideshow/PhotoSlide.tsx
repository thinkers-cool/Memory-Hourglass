import { mediaSrc, useDecodedImage } from "../../lib/slideshow/imageDecode";
import type { AssetCard } from "../../types";

export function PhotoSlide({ card }: { card: AssetCard }) {
  const src = mediaSrc(card);
  const ready = useDecodedImage(src);

  return (
    <img
      src={src}
      alt={card.file_name}
      draggable={false}
      className={`h-full w-full object-cover ${ready ? "" : "invisible"}`}
    />
  );
}
