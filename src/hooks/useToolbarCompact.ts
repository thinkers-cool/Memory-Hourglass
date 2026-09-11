import { useEffect, useState } from "react";

export const TOOLBAR_COMPACT_WIDTH = 720;

export function useToolbarCompact(
  rootRef: React.RefObject<HTMLElement | null>,
  threshold = TOOLBAR_COMPACT_WIDTH,
) {
  const [compact, setCompact] = useState(false);

  useEffect(() => {
    const element = rootRef.current;
    if (!element) return;

    const update = (width: number) => {
      setCompact(width > 0 && width < threshold);
    };

    update(element.getBoundingClientRect().width);

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      update(entry.contentRect.width);
    });
    observer.observe(element);

    return () => observer.disconnect();
  }, [rootRef, threshold]);

  return compact;
}
