import { convertFileSrc } from "@tauri-apps/api/core";
import { useRef } from "react";

export function VideoPoster({
  src,
  className,
  label,
}: {
  src: string;
  className?: string;
  label?: string;
}) {
  const seekedRef = useRef(false);

  return (
    <video
      src={convertFileSrc(src)}
      className={className}
      preload="metadata"
      muted
      playsInline
      aria-label={label}
      onLoadedMetadata={(event) => {
        if (seekedRef.current) return;
        seekedRef.current = true;
        const video = event.currentTarget;
        if (video.currentTime === 0) {
          video.currentTime = 0.001;
        }
      }}
    />
  );
}
