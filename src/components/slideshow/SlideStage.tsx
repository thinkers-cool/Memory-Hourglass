import { BlurredBackdrop } from "./BlurredBackdrop";
import { PhotoSlide } from "./PhotoSlide";
import { VideoSlide } from "./VideoSlide";
import { layerMotion } from "../../lib/slideshow/motion";
import { kenBurnsVariantForIndex } from "../../lib/slideshow/timing";
import type {
  SlideshowIntervalMs,
  SlideshowTheme,
} from "../../lib/slideshow/types";
import type { AssetCard } from "../../types";

function SlideLayer({
  card,
  theme,
  role,
  crossfadeProgress,
  slideElapsedMs,
  dwellMs,
  playing,
  muted,
  onVideoEnded,
}: {
  card: AssetCard;
  theme: SlideshowTheme;
  role: "outgoing" | "incoming";
  crossfadeProgress: number;
  slideElapsedMs: number;
  dwellMs: SlideshowIntervalMs;
  playing: boolean;
  muted: boolean;
  onVideoEnded: () => void;
}) {
  const motion = layerMotion({
    theme,
    role,
    crossfadeT: crossfadeProgress,
    slideElapsedMs,
    dwellMs,
    variant: kenBurnsVariantForIndex(card.id),
  });
  const isPhoto = card.kind !== "video";

  return (
    <div
      className="absolute inset-0 isolate overflow-hidden"
      style={{
        opacity: motion.opacity,
        transform: motion.transform,
        transformOrigin: "center center",
        zIndex: role === "incoming" ? 2 : 1,
        willChange: "opacity, transform",
      }}
    >
      {isPhoto ? (
        <PhotoSlide card={card} />
      ) : (
        <VideoSlide
          card={card}
          playing={playing && role === "incoming"}
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
  dwellMs,
  playing,
  incomingElapsedMs,
  outgoingElapsedMs,
  muted,
  onVideoEnded,
}: {
  items: AssetCard[];
  fromIndex: number | null;
  toIndex: number;
  progress: number;
  theme: SlideshowTheme;
  dwellMs: SlideshowIntervalMs;
  playing: boolean;
  incomingElapsedMs: number;
  outgoingElapsedMs: number;
  muted: boolean;
  onVideoEnded: () => void;
}) {
  const incoming = items[toIndex];
  const outgoing = fromIndex !== null ? items[fromIndex] : null;
  const transitioning = fromIndex !== null && fromIndex !== toIndex;
  const backdropCard = incoming ?? outgoing;

  if (!backdropCard) return null;

  const backdropStrength = 0.35;

  return (
    <div className="relative h-full w-full overflow-hidden bg-black">
      {transitioning && outgoing ? (
        <>
          <BlurredBackdrop
            card={outgoing}
            opacity={backdropStrength * (1 - progress)}
          />
          {incoming && (
            <BlurredBackdrop
              card={incoming}
              opacity={backdropStrength * progress}
            />
          )}
        </>
      ) : (
        <BlurredBackdrop card={backdropCard} opacity={backdropStrength} />
      )}
      {transitioning && outgoing && (
        <SlideLayer
          card={outgoing}
          theme={theme}
          role="outgoing"
          crossfadeProgress={progress}
          slideElapsedMs={outgoingElapsedMs}
          dwellMs={dwellMs}
          playing={false}
          muted={muted}
          onVideoEnded={onVideoEnded}
        />
      )}
      {incoming && (
        <SlideLayer
          card={incoming}
          theme={theme}
          role="incoming"
          crossfadeProgress={transitioning ? progress : 1}
          slideElapsedMs={incomingElapsedMs}
          dwellMs={dwellMs}
          playing={playing}
          muted={muted}
          onVideoEnded={onVideoEnded}
        />
      )}
    </div>
  );
}
