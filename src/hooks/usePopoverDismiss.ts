import { useEffect } from "react";

export function usePopoverDismiss({
  open,
  onClose,
  containerRef,
  anchorRef,
}: {
  open: boolean;
  onClose: () => void;
  containerRef: React.RefObject<HTMLElement | null>;
  anchorRef?: React.RefObject<HTMLElement | null>;
}) {
  useEffect(() => {
    if (!open) return;

    const onMouseDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (containerRef.current?.contains(target)) return;
      if (anchorRef?.current?.contains(target)) return;
      onClose();
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    document.addEventListener("mousedown", onMouseDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onMouseDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open, onClose, containerRef, anchorRef]);
}
