import { useRef, useState } from "react";
import { createPortal } from "react-dom";
import { HexColorPicker } from "react-colorful";
import { normalizeTagColor } from "../../lib/libraryIndicators";
import { usePopoverDismiss } from "../../hooks/usePopoverDismiss";
import { useAnchoredPopoverPlacement } from "./useAnchoredPopover";
import { IconTooltip } from "./Tooltip";

export function ColorPickerPopover({
  value,
  onChange,
  disabled,
  title = "Pick color",
}: {
  value: string;
  onChange: (color: string) => void;
  disabled?: boolean;
  title?: string;
}) {
  const [open, setOpen] = useState(false);
  const anchorRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const color = normalizeTagColor(value);
  const placement = useAnchoredPopoverPlacement({
    open,
    anchorRef,
    preferredWidth: 200,
    preferredHeight: 168,
  });

  usePopoverDismiss({
    open,
    onClose: () => setOpen(false),
    containerRef: popoverRef,
    anchorRef,
  });

  return (
    <>
      <IconTooltip tip={title} placement="bottom">
        <button
          ref={anchorRef}
          type="button"
          className="btn btn-ghost btn-interactive btn-xs btn-square h-8 min-h-0 w-8 shrink-0"
          aria-label={title}
          disabled={disabled}
          onClick={() => setOpen((prev) => !prev)}
        >
        <span
          className="inline-block h-3.5 w-3.5 rounded-full border border-interactive-border"
          style={{ backgroundColor: color }}
        />
      </button>
      </IconTooltip>
      {open &&
        placement &&
        createPortal(
          <div
            ref={popoverRef}
            className="surface-popover fixed z-[80] p-2 [&_.react-colorful]:h-[140px] [&_.react-colorful]:w-full"
            style={{
              top: placement.top,
              left: placement.left,
              width: placement.width,
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <HexColorPicker
              color={color}
              onChange={(next) => onChange(normalizeTagColor(next))}
            />
          </div>,
          document.body,
        )}
    </>
  );
}
