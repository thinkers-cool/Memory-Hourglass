import type { KenBurnsVariant } from "../../lib/slideshow/types";
import { useProgressiveImage } from "./BlurredBackdrop";
import type { AssetCard } from "../../types";

const KEN_BURNS_CLASS: Record<KenBurnsVariant, string> = {
  "zoom-in": "animate-ken-burns-in motion-reduce:animate-none",
  "zoom-out": "animate-ken-burns-out motion-reduce:animate-none",
  "pan-left": "animate-ken-burns-left motion-reduce:animate-none",
  "pan-right": "animate-ken-burns-right motion-reduce:animate-none",
};

export function PhotoSlide({
  card,
  kenBurns,
  kenBurnsVariant,
  dwellMs,
  animate,
}: {
  card: AssetCard;
  kenBurns: boolean;
  kenBurnsVariant: KenBurnsVariant;
  dwellMs: number;
  animate: boolean;
}) {
  const src = useProgressiveImage(card);
  const motionClass = kenBurns && animate ? KEN_BURNS_CLASS[kenBurnsVariant] : "";

  return (
    <img
      key={card.id}
      src={src}
      alt={card.file_name}
      draggable={false}
      className={`max-h-screen max-w-screen object-contain ${motionClass}`}
      style={
        kenBurns && animate
          ? { animationDuration: `${dwellMs}ms`, viewTransitionName: `asset-${card.id}` }
          : { viewTransitionName: `asset-${card.id}` }
      }
    />
  );
}
