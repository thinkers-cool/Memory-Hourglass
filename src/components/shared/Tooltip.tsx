import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

export const TOOLTIP_DELAY_MS = 600;

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

function suggestPlacement(anchor: DOMRect): TooltipPlacement {
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

function resolveTooltipCoords(
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

export function Tooltip({
  tip,
  placement = "top",
  delayMs = 0,
  multiline = false,
  className = "",
  children,
}: {
  tip?: string;
  placement?: TooltipPlacementInput;
  delayMs?: number;
  multiline?: boolean;
  className?: string;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const [style, setStyle] = useState<CSSProperties>({ visibility: "hidden" });
  const anchorRef = useRef<HTMLSpanElement>(null);
  const tipRef = useRef<HTMLDivElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, []);

  useLayoutEffect(() => {
    if (!open) {
      return;
    }

    const anchor = anchorRef.current?.getBoundingClientRect();
    const tipEl = tipRef.current;
    if (!anchor || !tipEl) {
      return;
    }

    const preferred =
      placement === "auto" ? suggestPlacement(anchor) : placement;
    const coords = resolveTooltipCoords(
      preferred,
      anchor,
      tipEl.offsetWidth,
      tipEl.offsetHeight,
    );

    setStyle({
      top: coords.top,
      left: coords.left,
      visibility: "visible",
    });
  }, [open, placement, tip, multiline]);

  useEffect(() => {
    if (!open) {
      return;
    }

    const reposition = () => {
      const anchor = anchorRef.current?.getBoundingClientRect();
      const tipEl = tipRef.current;
      if (!anchor || !tipEl) {
        return;
      }

      const preferred =
        placement === "auto" ? suggestPlacement(anchor) : placement;
      const coords = resolveTooltipCoords(
        preferred,
        anchor,
        tipEl.offsetWidth,
        tipEl.offsetHeight,
      );

      setStyle({
        top: coords.top,
        left: coords.left,
        visibility: "visible",
      });
    };

    window.addEventListener("scroll", reposition, true);
    window.addEventListener("resize", reposition);
    return () => {
      window.removeEventListener("scroll", reposition, true);
      window.removeEventListener("resize", reposition);
    };
  }, [open, placement, tip, multiline]);

  if (!tip) {
    return <>{children}</>;
  }

  const show = () => {
    if (delayMs > 0) {
      timerRef.current = setTimeout(() => setOpen(true), delayMs);
      return;
    }
    setOpen(true);
  };

  const hide = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = undefined;
    }
    setOpen(false);
    setStyle({ visibility: "hidden" });
  };

  return (
    <>
      <span
        ref={anchorRef}
        className={className}
        onMouseEnter={show}
        onMouseLeave={hide}
        onFocus={show}
        onBlur={hide}
      >
        {children}
      </span>
      {open
        ? createPortal(
            <div
              ref={tipRef}
              role="tooltip"
              className={`pointer-events-none fixed z-[200] rounded-[var(--radius-field)] bg-neutral px-2 py-1 text-xs text-neutral-content shadow-md ${
                multiline
                  ? "max-w-xs whitespace-normal break-all text-left"
                  : "whitespace-nowrap"
              }`}
              style={style}
            >
              {tip}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}

export function IconTooltip({
  tip,
  placement = "auto",
  className = "",
  children,
}: {
  tip?: string;
  placement?: TooltipPlacementInput;
  className?: string;
  children: ReactNode;
}) {
  return (
    <Tooltip
      tip={tip}
      placement={placement}
      delayMs={TOOLTIP_DELAY_MS}
      className={className || "inline-flex"}
    >
      {children}
    </Tooltip>
  );
}
