export type TooltipPlacement = "top" | "bottom" | "left" | "right";
export type TooltipPlacementInput = TooltipPlacement | "auto";

const VIEWPORT_MARGIN = 8;
const TOOLTIP_GAP = 8;

const OPPOSITE: Record<TooltipPlacement, TooltipPlacement> = {
  top: "bottom",
  bottom: "top",
  left: "right",
  right: "left",
};

export function suggestPlacement(anchor: DOMRect): TooltipPlacement {
  const vw = window.innerWidth;
  const vh = window.innerHeight;

  if (anchor.left < vw * 0.14) {
    return "right";
  }
  if (anchor.right > vw * 0.86) {
    return "left";
  }
  if (anchor.top < vh * 0.14) {
    return "bottom";
  }
  if (anchor.bottom > vh * 0.86) {
    return "top";
  }
  return "top";
}

function tooltipCoords(
  placement: TooltipPlacement,
  anchor: DOMRect,
  tipWidth: number,
  tipHeight: number,
): { top: number; left: number } {
  switch (placement) {
    case "right":
      return {
        top: anchor.top + anchor.height / 2 - tipHeight / 2,
        left: anchor.right + TOOLTIP_GAP,
      };
    case "left":
      return {
        top: anchor.top + anchor.height / 2 - tipHeight / 2,
        left: anchor.left - TOOLTIP_GAP - tipWidth,
      };
    case "bottom":
      return {
        top: anchor.bottom + TOOLTIP_GAP,
        left: anchor.left + anchor.width / 2 - tipWidth / 2,
      };
    default:
      return {
        top: anchor.top - TOOLTIP_GAP - tipHeight,
        left: anchor.left + anchor.width / 2 - tipWidth / 2,
      };
  }
}

function overflowsViewport(
  top: number,
  left: number,
  tipWidth: number,
  tipHeight: number,
): boolean {
  return (
    left < VIEWPORT_MARGIN ||
    top < VIEWPORT_MARGIN ||
    left + tipWidth > window.innerWidth - VIEWPORT_MARGIN ||
    top + tipHeight > window.innerHeight - VIEWPORT_MARGIN
  );
}

function clampToViewport(
  top: number,
  left: number,
  tipWidth: number,
  tipHeight: number,
): { top: number; left: number } {
  return {
    top: Math.min(
      Math.max(top, VIEWPORT_MARGIN),
      window.innerHeight - tipHeight - VIEWPORT_MARGIN,
    ),
    left: Math.min(
      Math.max(left, VIEWPORT_MARGIN),
      window.innerWidth - tipWidth - VIEWPORT_MARGIN,
    ),
  };
}

export function resolveTooltipCoords(
  preferred: TooltipPlacement,
  anchor: DOMRect,
  tipWidth: number,
  tipHeight: number,
): { top: number; left: number } {
  const candidates: TooltipPlacement[] = [
    preferred,
    OPPOSITE[preferred],
    "bottom",
    "top",
    "right",
    "left",
  ];
  const seen = new Set<TooltipPlacement>();

  for (const placement of candidates) {
    if (seen.has(placement)) {
      continue;
    }
    seen.add(placement);
    const coords = tooltipCoords(placement, anchor, tipWidth, tipHeight);
    if (!overflowsViewport(coords.top, coords.left, tipWidth, tipHeight)) {
      return coords;
    }
  }

  const fallback = tooltipCoords(preferred, anchor, tipWidth, tipHeight);
  return clampToViewport(fallback.top, fallback.left, tipWidth, tipHeight);
}

export function resolveTooltipLayout(
  placement: TooltipPlacementInput,
  anchor: DOMRect | undefined,
  tipWidth: number,
  tipHeight: number,
): { top: number; left: number } | null {
  if (!anchor || tipWidth <= 0 || tipHeight <= 0) {
    return null;
  }
  const preferred = placement === "auto" ? suggestPlacement(anchor) : placement;
  return resolveTooltipCoords(preferred, anchor, tipWidth, tipHeight);
}
