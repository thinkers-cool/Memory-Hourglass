import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef } from "react";
import type { AssetCard } from "../../types";

export function VideoSlide({
  card,
  playing,
  muted,
  onEnded,
}: {
  card: AssetCard;
  playing: boolean;
  muted: boolean;
  onEnded: () => void;
}) {
  const ref = useRef<HTMLVideoElement>(null);
  const src = convertFileSrc(card.abs_path);

  useEffect(() => {
    const video = ref.current!;
    if (playing) {
      void video.play().catch(() => undefined);
    } else {
      video.pause();
    }
  }, [playing, card.id]);

  useEffect(() => {
    const video = ref.current!;
    video.currentTime = 0;
    if (playing) {
      void video.play().catch(() => undefined);
    }
  }, [card.id, playing]);

  return (
    <video
      ref={ref}
      src={src}
      muted={muted}
      playsInline
      className="max-h-screen max-w-screen object-contain"
      onEnded={onEnded}
    />
  );
}
