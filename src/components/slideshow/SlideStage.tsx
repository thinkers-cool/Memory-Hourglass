import { BlurredBackdrop } from "./BlurredBackdrop";
import { PhotoSlide } from "./PhotoSlide";
import { VideoSlide } from "./VideoSlide";
import { kenBurnsVariantForIndex } from "../../lib/slideshow/timing";
import { layerStyle } from "../../lib/slideshow/transitions";
import type { SlideshowIntervalMs, SlideshowTheme } from "../../lib/slideshow/types";
import type { AssetCard } from "../../types";

function SlideLayer({
  card,
  theme,
  role,
  progress,
  dwellMs,
  kenBurns,
  playing,
  muted,
  transitioning,
  onVideoEnded,
}: {
  card: AssetCard;
  theme: SlideshowTheme;
  role: "outgoing" | "incoming";
  progress: number;
  dwellMs: SlideshowIntervalMs;
  kenBurns: boolean;
  playing: boolean;
  muted: boolean;
  transitioning: boolean;
  onVideoEnded: () => void;
}) {
  const style = layerStyle(theme, role, progress);
  const isPhoto = card.kind !== "video";
  const showKenBurns =
    kenBurns && isPhoto && role === "incoming" && !transitioning && playing;

  return (
    <div
      className="absolute inset-0 flex items-center justify-center will-change-[opacity,transform]"
      style={{
        opacity: style.opacity,
        transform: style.transform,
        zIndex: role === "incoming" ? 2 : 1,
      }}
    >
      {isPhoto ? (
        <PhotoSlide
          card={card}
          kenBurns={kenBurns}
          kenBurnsVariant={kenBurnsVariantForIndex(card.id)}
          dwellMs={dwellMs}
          animate={showKenBurns && playing}
        />
      ) : (
        <VideoSlide
          card={card}
          playing={playing && role === "incoming" && !transitioning}
          muted={muted}
          onEnded={onVideoEnded}
        />
      )}
    </div>
  );
}

export function SlideStage({
  items,
  fromIndex,
  toIndex,
  progress,
  theme,
  kenBurns,
  dwellMs,
  playing,
  muted,
  onVideoEnded,
}: {
  items: AssetCard[];
  fromIndex: number | null;
  toIndex: number;
  progress: number;
  theme: SlideshowTheme;
  kenBurns: boolean;
  dwellMs: SlideshowIntervalMs;
  playing: boolean;
  muted: boolean;
  onVideoEnded: () => void;
}) {
  const incoming = items[toIndex];
  const outgoing = fromIndex !== null ? items[fromIndex] : null;
  const transitioning = fromIndex !== null && fromIndex !== toIndex && progress < 1;
  const backdropCard = incoming ?? outgoing;

  if (!backdropCard) return null;

  return (
    <div className="relative h-full w-full overflow-hidden bg-black">
      <BlurredBackdrop card={backdropCard} />
      {transitioning && outgoing && (
        <SlideLayer
          card={outgoing}
          theme={theme}
          role="outgoing"
          progress={progress}
          dwellMs={dwellMs}
          kenBurns={kenBurns}
          playing={false}
          muted={muted}
          transitioning={true}
          onVideoEnded={onVideoEnded}
        />
      )}
      {incoming && (
        <SlideLayer
          card={incoming}
          theme={theme}
          role="incoming"
          progress={transitioning ? progress : 1}
          dwellMs={dwellMs}
          kenBurns={kenBurns}
          playing={playing}
          muted={muted}
          transitioning={transitioning}
          onVideoEnded={onVideoEnded}
        />
      )}
    </div>
  );
}
