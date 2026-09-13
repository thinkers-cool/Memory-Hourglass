import { useEffect, useRef, useState, type MutableRefObject } from "react";

export function useSlideElapsed(
  index: number,
  playing: boolean,
  elapsedRef?: MutableRefObject<number>,
): number {
  const anchorRef = useRef({
    startedAt: performance.now(),
    pausedTotal: 0,
    pauseAt: 0,
  });
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    anchorRef.current = {
      startedAt: performance.now(),
      pausedTotal: 0,
      pauseAt: 0,
    };
    setElapsed(0);
    if (elapsedRef) elapsedRef.current = 0;
  }, [index, elapsedRef]);

  useEffect(() => {
    const anchor = anchorRef.current;
    if (!playing) {
      if (anchor.pauseAt === 0) anchor.pauseAt = performance.now();
      return;
    }

    if (anchor.pauseAt > 0) {
      anchor.pausedTotal += performance.now() - anchor.pauseAt;
      anchor.pauseAt = 0;
    }

    let raf = 0;
    const tick = () => {
      const current = anchorRef.current;
      const pausedNow =
        current.pauseAt > 0 ? performance.now() - current.pauseAt : 0;
      const next =
        performance.now() - current.startedAt - current.pausedTotal - pausedNow;
      setElapsed(next);
      if (elapsedRef) elapsedRef.current = next;
      raf = requestAnimationFrame(tick);
    };

    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [playing, index, elapsedRef]);

  return elapsed;
}
