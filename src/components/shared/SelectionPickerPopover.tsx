import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { usePopoverDismiss } from "../../hooks/usePopoverDismiss";
import { ghostBtnClass } from "../../lib/buttonClass";
import {
  INPUT_CONTROL_FULL_CLASS,
  MENU_ITEM_BUTTON_CLASS,
  MENU_PICKER_LIST_CLASS,
} from "../../lib/formControlClass";
import { menuPickerItemClass } from "../../lib/optionRowClass";

export interface PickerItem {
  key: string;
  label: string;
  icon?: React.ReactNode;
}

const TRIGGER_CLASS = ghostBtnClass(
  "btn-xs h-7 min-h-0 px-2.5 text-xs font-normal",
);

const INPUT_CLASS = `${INPUT_CONTROL_FULL_CLASS} h-7 px-2 text-xs`;

export function SelectionPickerPopover({
  label,
  shortcut,
  items,
  placeholder,
  busy,
  open,
  onOpenChange,
  onToggle,
  onCreate,
  selectedKeys,
}: {
  label: string;
  shortcut?: string;
  items: PickerItem[];
  placeholder: string;
  busy?: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  onToggle: (key: string, active: boolean) => void | Promise<void>;
  onCreate?: (value: string) => string | void | Promise<string | void>;
  selectedKeys?: readonly string[];
}) {
  const { t } = useTranslation("common");
  const [hoverOpen, setHoverOpen] = useState(false);
  const [internalPinned, setInternalPinned] = useState(false);
  const [input, setInput] = useState("");
  const [sessionActiveKeys, setSessionActiveKeys] = useState<Set<string>>(
    new Set(),
  );
  const rootRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const pinned = open === true || internalPinned;
  const visible = hoverOpen || pinned;
  const usesSelectedKeys = selectedKeys !== undefined;

  const isItemActive = (key: string) => {
    if (usesSelectedKeys) {
      return selectedKeys.includes(key);
    }
    return sessionActiveKeys.has(key);
  };

  const close = () => {
    setHoverOpen(false);
    setInternalPinned(false);
    setSessionActiveKeys(new Set());
    setInput("");
    onOpenChange?.(false);
  };

  const panelRef = useRef<HTMLDivElement>(null);

  usePopoverDismiss({
    open: visible,
    onClose: close,
    containerRef: panelRef,
    anchorRef: rootRef,
  });

  useEffect(() => {
    if (!open) {
      setInternalPinned(false);
      setSessionActiveKeys(new Set());
      setInput("");
    }
  }, [open]);

  useEffect(() => {
    if (usesSelectedKeys) {
      setSessionActiveKeys(new Set());
    }
  }, [usesSelectedKeys, selectedKeys]);

  useEffect(() => {
    if (pinned && inputRef.current) {
      inputRef.current.focus();
    }
  }, [pinned]);

  const handleItemClick = (key: string) => {
    const nextActive = !isItemActive(key);
    if (!usesSelectedKeys) {
      setSessionActiveKeys((prev) => {
        const next = new Set(prev);
        if (nextActive) next.add(key);
        else next.delete(key);
        return next;
      });
    }
    void onToggle(key, nextActive);
  };

  const handleCreate = async () => {
    const trimmed = input.trim();
    if (!trimmed || busy) return;
    const key = onCreate ? await onCreate(trimmed) : trimmed;
    const resolvedKey = key ?? trimmed;
    if (!usesSelectedKeys) {
      setSessionActiveKeys((prev) => new Set(prev).add(resolvedKey));
    }
    setInput("");
  };

  const pinOpen = () => {
    setHoverOpen(false);
    setInternalPinned(true);
    onOpenChange?.(true);
  };

  return (
    <div
      ref={rootRef}
      className="relative"
      onMouseEnter={() => setHoverOpen(true)}
      onMouseLeave={() => {
        if (!pinned) setHoverOpen(false);
      }}
    >
      <button type="button" className={TRIGGER_CLASS} onClick={pinOpen}>
        {label}
        {shortcut && (
          <span className="ml-1 text-[10px] font-normal uppercase tracking-wide text-content-faint">
            {shortcut}
          </span>
        )}
      </button>

      {visible && (
        <div
          ref={panelRef}
          className="absolute bottom-full left-1/2 z-[60] min-w-[220px] -translate-x-1/2 pb-4"
          onMouseEnter={() => setHoverOpen(true)}
        >
          <div
            className="surface-popover p-1.5"
            onClick={(e) => e.stopPropagation()}
          >
            <input
              ref={inputRef}
              type="text"
              className={`${INPUT_CLASS} mb-1.5`}
              placeholder={placeholder}
              value={input}
              disabled={busy}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void handleCreate();
                if (e.key === "Escape") close();
              }}
            />

            <ul className={`${MENU_PICKER_LIST_CLASS} max-h-40`}>
              {items.length === 0 ? (
                <li className="pointer-events-none px-2 py-1 text-xs opacity-45">
                  {t("empty.noneYet")}
                </li>
              ) : (
                items.map((item) => (
                  <li key={item.key}>
                    <button
                      type="button"
                      disabled={busy}
                      className={`${MENU_ITEM_BUTTON_CLASS} ${menuPickerItemClass(isItemActive(item.key))}`}
                      onClick={() => handleItemClick(item.key)}
                    >
                      <span className="flex w-4 shrink-0 items-center justify-center">
                        {item.icon ?? <span className="w-3" />}
                      </span>
                      <span className="truncate">{item.label}</span>
                    </button>
                  </li>
                ))
              )}
            </ul>
          </div>
        </div>
      )}
    </div>
  );
}
