import { useLayoutEffect, useState, type RefObject } from "react";
import {
  computeAnchoredPopoverPlacement,
  type AnchoredPopoverPlacement,
} from "../../lib/anchoredPopover";

export type { AnchoredPopoverPlacement };

type UseAnchoredPopoverOptions = {
  open: boolean;
  anchorRef: RefObject<HTMLElement | null>;
  preferredWidth?: number;
  preferredHeight?: number;
  margin?: number;
  boundarySelector?: string;
};

export function useAnchoredPopoverPlacement({
  open,
  anchorRef,
  preferredWidth = 236,
  preferredHeight = 248,
  margin = 8,
  boundarySelector = "[data-library-panel]",
}: UseAnchoredPopoverOptions): AnchoredPopoverPlacement | null {
  const [placement, setPlacement] = useState<AnchoredPopoverPlacement | null>(
    null,
  );

  useLayoutEffect(() => {
    if (!open || !anchorRef.current) {
      setPlacement(null);
      return;
    }

    const compute = () => {
      const anchor = anchorRef.current!.getBoundingClientRect();
      const boundary = anchorRef.current!.closest(
        boundarySelector,
      ) as HTMLElement | null;
      const boundaryRect = boundary?.getBoundingClientRect();

      setPlacement(
        computeAnchoredPopoverPlacement({
          anchorLeft: anchor.left,
          anchorTop: anchor.top,
          anchorBottom: anchor.bottom,
          boundaryLeft: boundaryRect?.left ?? margin,
          boundaryRight: boundaryRect?.right ?? window.innerWidth - margin,
          boundaryTop: boundaryRect?.top ?? margin,
          boundaryBottom: boundaryRect?.bottom ?? window.innerHeight - margin,
          preferredWidth,
          preferredHeight,
          margin,
        }),
      );
    };

    compute();
    const frame = requestAnimationFrame(compute);

    window.addEventListener("resize", compute);
    window.addEventListener("scroll", compute, true);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", compute);
      window.removeEventListener("scroll", compute, true);
    };
  }, [
    open,
    anchorRef,
    preferredWidth,
    preferredHeight,
    margin,
    boundarySelector,
  ]);

  return placement;
}
