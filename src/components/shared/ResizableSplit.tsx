import { useCallback, useEffect, useRef, useState } from "react";
import {
  clampLeadingPanelWidth,
  clampTrailingPanelWidth,
} from "../../lib/panelResize";

const PANEL_TRANSITION_MS = 300;

export { PANEL_TRANSITION_MS };

export function ResizableSplit({
  leading,
  trailing,
  leadingWidth,
  onLeadingWidthChange,
  minLeading,
  minTrailing,
}: {
  leading: React.ReactNode;
  trailing: React.ReactNode;
  leadingWidth: number;
  onLeadingWidthChange: (width: number) => void;
  minLeading: number;
  minTrailing: number;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);

  const onPointerDown = useCallback((event: React.PointerEvent) => {
    event.preventDefault();
    setDragging(true);
    event.currentTarget.setPointerCapture(event.pointerId);
  }, []);

  useEffect(() => {
    if (!dragging) return;

    const onPointerMove = (event: PointerEvent) => {
      const container = containerRef.current!;
      const rect = container.getBoundingClientRect();
      const nextWidth = event.clientX - rect.left;
      onLeadingWidthChange(
        clampLeadingPanelWidth(nextWidth, rect.width, minLeading, minTrailing),
      );
    };

    const onPointerUp = () => setDragging(false);

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    };
  }, [dragging, minLeading, minTrailing, onLeadingWidthChange]);

  return (
    <div
      ref={containerRef}
      className={`flex min-h-0 min-w-0 flex-1 ${dragging ? "select-none" : ""}`}
    >
      <div className="shrink-0 overflow-auto" style={{ width: leadingWidth }}>
        {leading}
      </div>

      <div
        className="group relative z-10 w-1 shrink-0 cursor-col-resize bg-split-idle hover:bg-split-hover active:bg-split-active"
        onPointerDown={onPointerDown}
      >
        <div className="absolute inset-y-0 -left-1 -right-1" />
      </div>

      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">{trailing}</div>
    </div>
  );
}

export function ResizableTrailingPanel({
  main,
  side,
  sideWidth,
  onSideWidthChange,
  minMain,
  minSide,
  showSide,
}: {
  main: React.ReactNode;
  side?: React.ReactNode;
  sideWidth: number;
  onSideWidthChange: (width: number) => void;
  minMain: number;
  minSide: number;
  showSide: boolean;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);
  const [renderSide, setRenderSide] = useState(showSide);
  const [open, setOpen] = useState(false);
  const lastSideRef = useRef(side);
  if (side) lastSideRef.current = side;
  const sideContent = side ?? lastSideRef.current;
  const wasOpenRef = useRef(showSide);

  useEffect(() => {
    if (showSide) {
      setRenderSide(true);
      if (!wasOpenRef.current) {
        setOpen(false);
        const frame = requestAnimationFrame(() => {
          requestAnimationFrame(() => setOpen(true));
        });
        wasOpenRef.current = true;
        return () => cancelAnimationFrame(frame);
      }
      setOpen(true);
      wasOpenRef.current = true;
      return;
    }
    setOpen(false);
    wasOpenRef.current = false;
  }, [showSide]);

  const onSideTransitionEnd = useCallback(
    (event: React.TransitionEvent<HTMLDivElement>) => {
      if (event.propertyName !== "width") return;
      if (!open) setRenderSide(false);
    },
    [open],
  );

  const onPointerDown = useCallback((event: React.PointerEvent) => {
    event.preventDefault();
    setDragging(true);
    event.currentTarget.setPointerCapture(event.pointerId);
  }, []);

  useEffect(() => {
    if (!dragging) return;

    const onPointerMove = (event: PointerEvent) => {
      const container = containerRef.current!;
      const rect = container.getBoundingClientRect();
      const nextWidth = rect.right - event.clientX;
      onSideWidthChange(
        clampTrailingPanelWidth(nextWidth, rect.width, minMain, minSide),
      );
    };

    const onPointerUp = () => setDragging(false);

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    };
  }, [dragging, minMain, minSide, onSideWidthChange]);

  return (
    <div
      ref={containerRef}
      className={`relative z-0 flex min-h-0 min-w-0 flex-1 ${dragging ? "select-none" : ""}`}
    >
      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">{main}</div>

      {renderSide && sideContent && (
        <>
          <div
            className={`group relative z-10 w-1 shrink-0 cursor-col-resize bg-split-idle hover:bg-split-hover active:bg-split-active ${
              open ? "opacity-100" : "pointer-events-none opacity-0"
            } ${dragging ? "" : "transition-opacity duration-300 ease-in-out"}`}
            onPointerDown={onPointerDown}
          >
            <div className="absolute inset-y-0 -left-1 -right-1" />
          </div>
          <div
            className={`flex min-h-0 shrink-0 flex-col overflow-hidden ${
              dragging ? "" : "transition-[width] ease-in-out"
            }`}
            style={{
              width: open ? sideWidth : 0,
              transitionDuration: dragging ? undefined : `${PANEL_TRANSITION_MS}ms`,
            }}
            onTransitionEnd={onSideTransitionEnd}
          >
            <div className="h-full shrink-0" style={{ width: sideWidth }}>
              {sideContent}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
