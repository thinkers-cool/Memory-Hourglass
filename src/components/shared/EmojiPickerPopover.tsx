import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { loadEmojiCatalog } from "../../lib/emojiCatalog";
import { usePopoverDismiss } from "../../hooks/usePopoverDismiss";
import { useAnchoredPopoverPlacement } from "./useAnchoredPopover";

export function EmojiPickerPopover({
  value,
  onChange,
  disabled,
  title = "Pick emoji",
}: {
  value: string;
  onChange: (emoji: string) => void;
  disabled?: boolean;
  title?: string;
}) {
  const [open, setOpen] = useState(false);
  const [emojis, setEmojis] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const anchorRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const placement = useAnchoredPopoverPlacement({
    open,
    anchorRef,
    preferredWidth: 192,
    preferredHeight: 220,
  });

  usePopoverDismiss({
    open,
    onClose: () => setOpen(false),
    containerRef: popoverRef,
    anchorRef,
  });

  useEffect(() => {
    if (!open || emojis.length > 0) return;
    setLoading(true);
    void loadEmojiCatalog()
      .then(setEmojis)
      .finally(() => setLoading(false));
  }, [open, emojis.length]);

  const handlePick = (emoji: string) => {
    onChange(emoji);
    setOpen(false);
  };

  return (
    <>
      <button
        ref={anchorRef}
        type="button"
        className="btn btn-ghost btn-interactive btn-xs btn-square h-8 min-h-0 w-8 shrink-0 text-base"
        title={title}
        disabled={disabled}
        onClick={() => setOpen((prev) => !prev)}
      >
        {value || "😀"}
      </button>
      {open &&
        placement &&
        createPortal(
          <div
            ref={popoverRef}
            className="surface-popover fixed z-[80] overflow-hidden"
            style={{
              top: placement.top,
              left: placement.left,
              width: placement.width,
              maxHeight: 220,
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="max-h-[220px] overflow-y-auto p-2">
              {loading ? (
                <div className="flex h-20 items-center justify-center">
                  <span className="loading loading-spinner loading-sm" />
                </div>
              ) : (
                <div className="grid grid-cols-6 gap-1">
                  {emojis.map((emoji) => (
                    <button
                      key={emoji}
                      type="button"
                      className="btn btn-ghost btn-interactive btn-xs h-7 min-h-0 w-7 text-base"
                      onClick={() => handlePick(emoji)}
                    >
                      {emoji}
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>,
          document.body,
        )}
    </>
  );
}
