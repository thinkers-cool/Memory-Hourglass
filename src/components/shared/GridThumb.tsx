import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import {
  acquireThumbLoadSlot,
  releaseThumbLoadSlot,
} from "../../lib/thumbLoad";

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
    const host = hostRef.current;
    if (!host) {
      return;
    }
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "240px" },
    );
    observer.observe(host);
    return () => observer.disconnect();
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
    void acquireThumbLoadSlot().then(() => {
      if (cancelled) {
        releaseThumbLoadSlot();
        return;
      }
      slotGranted = true;
      setSrc(convertFileSrc(thumbPath));
    });
    return () => {
      cancelled = true;
      if (slotGranted && !settledRef.current) {
        releaseThumbLoadSlot();
        settledRef.current = true;
      }
    };
  }, [visible, src, thumbPath]);

  const settleLoad = () => {
    if (settledRef.current) {
      return;
    }
    settledRef.current = true;
    releaseThumbLoadSlot();
  };

  return (
    <span ref={hostRef} className="block h-full w-full">
      {src ? (
        <img
          src={src}
          alt={alt}
          className={className}
          loading="lazy"
          decoding="async"
          onLoad={settleLoad}
          onError={settleLoad}
        />
      ) : (
        <span className={`block ${className} bg-surface-inset-strong`} aria-hidden />
      )}
    </span>
  );
}
