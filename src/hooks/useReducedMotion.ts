import { useEffect, useLayoutEffect, useRef, useState } from "react";

export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(false);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setReduced(mq.matches);
    update();
    mq.addEventListener("change", update);
    return () => mq.removeEventListener("change", update);
  }, []);

  return reduced;
}

export function useRafTransition(
  active: boolean,
  durationMs: number,
  onComplete: () => void,
): number {
  const [progress, setProgress] = useState(1);
  const rafRef = useRef(0);
  const startRef = useRef(0);
  const onCompleteRef = useRef(onComplete);
  onCompleteRef.current = onComplete;

  useLayoutEffect(() => {
    if (!active) return;
    setProgress(0);
    startRef.current = performance.now();
  }, [active, durationMs]);

  useEffect(() => {
    if (!active) {
      cancelAnimationFrame(rafRef.current);
      return;
    }

    const tick = (now: number) => {
      const elapsed = now - startRef.current;
      const next = durationMs <= 0 ? 1 : Math.min(1, elapsed / durationMs);
      setProgress(next);
      if (next < 1) {
        rafRef.current = requestAnimationFrame(tick);
      } else {
        onCompleteRef.current();
      }
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [active, durationMs]);

  return progress;
}
