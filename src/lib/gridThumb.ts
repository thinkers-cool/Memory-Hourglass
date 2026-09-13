import {
  acquireThumbLoadSlot,
  releaseThumbLoadSlot,
} from "./thumbLoad";

const INTERSECTION_ROOT_MARGIN = "240px";

export function bindThumbIntersection(
  host: HTMLElement | null,
  onVisible: () => void,
): () => void {
  if (!host) {
    return () => undefined;
  }
  const observer = new IntersectionObserver(
    ([entry]) => {
      if (entry.isIntersecting) {
        onVisible();
        observer.disconnect();
      }
    },
    { rootMargin: INTERSECTION_ROOT_MARGIN },
  );
  observer.observe(host);
  return () => observer.disconnect();
}

export function settleThumbLoad(settled: { current: boolean }): void {
  if (settled.current) {
    return;
  }
  settled.current = true;
  releaseThumbLoadSlot();
}

export async function loadThumbSrc(
  thumbPath: string,
  convert: (path: string) => string,
  isCancelled: () => boolean,
): Promise<string | null> {
  await acquireThumbLoadSlot();
  if (isCancelled()) {
    releaseThumbLoadSlot();
    return null;
  }
  return convert(thumbPath);
}
