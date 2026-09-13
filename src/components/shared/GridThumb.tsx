import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import {
  bindThumbIntersection,
  loadThumbSrc,
  settleThumbLoad,
} from "../../lib/gridThumb";
import { releaseThumbLoadSlot } from "../../lib/thumbLoad";

export function GridThumb({
  thumbPath,
  alt,
  className,
}: {
  thumbPath: string;
  alt: string;
  className: string;
}) {
  const hostRef = useRef<HTMLSpanElement>(null);
  const settledRef = useRef(false);
  const [src, setSrc] = useState<string | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    return bindThumbIntersection(hostRef.current, () => setVisible(true));
  }, [thumbPath]);

  useEffect(() => {
    settledRef.current = false;
  }, [thumbPath]);

  useEffect(() => {
    if (!visible || src) {
      return;
    }
    let cancelled = false;
    let slotGranted = false;
    void loadThumbSrc(thumbPath, convertFileSrc, () => cancelled).then(
      (nextSrc) => {
        if (nextSrc === null) {
          return;
        }
        slotGranted = true;
        setSrc(nextSrc);
      },
    );
    return () => {
      cancelled = true;
      if (slotGranted && !settledRef.current) {
        releaseThumbLoadSlot();
        settledRef.current = true;
      }
    };
  }, [visible, src, thumbPath]);

  return (
    <span ref={hostRef} className="block h-full w-full">
      {src ? (
        <img
          src={src}
          alt={alt}
          className={className}
          loading="lazy"
          decoding="async"
          onLoad={() => settleThumbLoad(settledRef)}
          onError={() => settleThumbLoad(settledRef)}
        />
      ) : (
        <span className={`block ${className} bg-surface-inset-strong`} aria-hidden />
      )}
    </span>
  );
}
