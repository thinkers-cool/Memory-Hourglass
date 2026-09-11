import { Palette } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { createPortal } from "react-dom";
import { useAppTheme, syncThemeController } from "../../hooks/useAppTheme";
import { usePopoverDismiss } from "../../hooks/usePopoverDismiss";
import { computeFloatingMenuPlacement } from "../../lib/anchoredPopover";
import { APP_THEMES, themeLabel, type AppTheme } from "../../lib/theme";
import { readAllThemePreviewColors, type ThemePreviewColors } from "../../lib/themePreview";
import { ghostBtnClass } from "../../lib/buttonClass";
import { optionRowClass } from "../../lib/optionRowClass";
import { ThemePreviewSwatches } from "./ThemePreviewSwatches";

export type ThemeMenuPlacement = "float-top-end" | "float-bottom-end";

const MENU_WIDTH = 240;
const MENU_MAX_HEIGHT = 320;

const TRIGGER_CLASS = {
  sm: ghostBtnClass("btn-sm btn-square h-8 min-h-0 w-8"),
  nav: ghostBtnClass("btn-square h-11 min-h-0 w-11"),
} as const;

const ICON_CLASS = {
  sm: "h-3.5 w-3.5 shrink-0",
  nav: "h-5 w-5 shrink-0",
} as const;

export function ThemeSwitcher({
  iconOnly = true,
  menuPlacement = "float-bottom-end",
  size = "sm",
}: {
  iconOnly?: boolean;
  menuPlacement?: ThemeMenuPlacement;
  size?: "sm" | "nav";
}) {
  const { t } = useTranslation("common");
  const { theme, setTheme } = useAppTheme();
  const [open, setOpen] = useState(false);
  const [menuPosition, setMenuPosition] = useState<{ top: number; left: number } | null>(
    null,
  );
  const [previewColors, setPreviewColors] = useState<Partial<Record<AppTheme, ThemePreviewColors>>>(
    {},
  );
  const anchorRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const floatingPlacement =
    menuPlacement === "float-top-end" ? "top-end" : "bottom-end";

  useEffect(() => {
    syncThemeController(theme);
  }, [theme]);

  useLayoutEffect(() => {
    setPreviewColors(readAllThemePreviewColors(APP_THEMES));
  }, []);

  const updateMenuPosition = useCallback(() => {
    if (!open || !anchorRef.current) {
      setMenuPosition(null);
      return;
    }
    const anchor = anchorRef.current.getBoundingClientRect();
    const menuEl = menuRef.current;
    let width = MENU_WIDTH;
    let height = MENU_MAX_HEIGHT;
    if (menuEl) {
      width = menuEl.offsetWidth;
      height = menuEl.offsetHeight;
    }
    setMenuPosition(
      computeFloatingMenuPlacement(
        anchor,
        { width, height },
        floatingPlacement,
      ),
    );
  }, [open, floatingPlacement]);

  useLayoutEffect(() => {
    updateMenuPosition();
    if (!open) return;
    const frame = requestAnimationFrame(updateMenuPosition);
    window.addEventListener("resize", updateMenuPosition);
    window.addEventListener("scroll", updateMenuPosition, true);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", updateMenuPosition);
      window.removeEventListener("scroll", updateMenuPosition, true);
    };
  }, [open, updateMenuPosition, previewColors]);

  usePopoverDismiss({
    open,
    onClose: () => setOpen(false),
    containerRef: menuRef,
    anchorRef,
  });

  const close = () => setOpen(false);

  return (
    <>
      <button
        ref={anchorRef}
        type="button"
        className={
          iconOnly
            ? TRIGGER_CLASS[size]
            : ghostBtnClass("btn-sm h-8 min-h-0 max-w-32 gap-1 truncate text-xs font-normal")
        }
        title={t("label.theme")}
        aria-label={t("aria.theme")}
        aria-expanded={open}
        onClick={() => setOpen((prev) => !prev)}
      >
        <Palette className={ICON_CLASS[size]} />
        {!iconOnly ? <span className="truncate">{themeLabel(theme)}</span> : null}
      </button>
      {open &&
        createPortal(
          <div
            ref={menuRef}
            className="surface-popover fixed z-[80] w-max p-2"
            style={
              menuPosition
                ? { top: menuPosition.top, left: menuPosition.left }
                : { top: -9999, left: -9999, visibility: "hidden" as const }
            }
            onClick={(event) => event.stopPropagation()}
          >
            <ul className="max-h-80 overflow-y-auto">
              {APP_THEMES.map((option) => (
                <li key={option}>
                  <label
                    className={`flex cursor-pointer items-center gap-2.5 rounded-[var(--radius-field)] px-2 py-1.5 ${optionRowClass(false)}`}
                  >
                    <input
                      type="radio"
                      name="memhg-theme"
                      className="radio radio-primary radio-sm shrink-0"
                      aria-label={themeLabel(option)}
                      value={option}
                      checked={theme === option}
                      onChange={() => {
                        setTheme(option);
                        close();
                      }}
                    />
                    <span className="min-w-0 flex-1 text-sm">{themeLabel(option)}</span>
                    <ThemePreviewSwatches theme={option} colors={previewColors[option]} />
                  </label>
                </li>
              ))}
            </ul>
          </div>,
          document.body,
        )}
    </>
  );
}
