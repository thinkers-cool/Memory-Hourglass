import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import {
  resolveTooltipLayout,
  type TooltipPlacementInput,
} from "../../lib/tooltipPlacement";

export const TOOLTIP_DELAY_MS = 600;

export type { TooltipPlacement, TooltipPlacementInput } from "../../lib/tooltipPlacement";

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

  const applyLayout = () => {
    const anchor = anchorRef.current?.getBoundingClientRect();
    const tipEl = tipRef.current;
    if (!anchor || !tipEl) {
      return;
    }
    const tipRect = tipEl.getBoundingClientRect();
    const tipWidth = tipRect.width || tipEl.offsetWidth || tipEl.scrollWidth;
    const tipHeight = tipRect.height || tipEl.offsetHeight || tipEl.scrollHeight;
    const coords = resolveTooltipLayout(
      placement,
      anchor,
      tipWidth,
      tipHeight,
    );
    if (!coords) {
      requestAnimationFrame(applyLayout);
      return;
    }
    setStyle({
      top: coords.top,
      left: coords.left,
      visibility: "visible",
    });
  };

  useLayoutEffect(() => {
    if (!open) {
      return;
    }
    applyLayout();
  }, [open, placement, tip, multiline]);

  useEffect(() => {
    if (!open) {
      return;
    }

    const reposition = () => {
      applyLayout();
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
