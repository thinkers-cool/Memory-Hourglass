import type { ThemePreviewColors } from "../../lib/themePreview";
import type { AppTheme } from "../../lib/theme";
import { THEME_PREVIEW_VARS } from "../../lib/themePreview";

function Swatch({ color }: { color: string }) {
  return (
    <span
      className="h-2.5 w-2.5 rounded-full ring-1 ring-base-content/10"
      style={{ backgroundColor: color }}
    />
  );
}

export function ThemePreviewSwatches({
  theme,
  colors,
}: {
  theme: AppTheme;
  colors?: ThemePreviewColors;
}) {
  if (colors?.background && colors.primary && colors.strong) {
    return (
      <span
        className="inline-flex shrink-0 items-center gap-0.5 rounded-full border border-divider-subtle p-0.5"
        aria-hidden
      >
        <Swatch color={colors.background} />
        <Swatch color={colors.primary} />
        <Swatch color={colors.strong} />
      </span>
    );
  }

  return (
    <span
      data-theme={theme}
      className="inline-flex shrink-0 items-center gap-0.5 rounded-full border border-divider-subtle p-0.5"
      aria-hidden
    >
      <span
        className="h-2.5 w-2.5 rounded-full ring-1 ring-base-content/10"
        style={{ backgroundColor: `var(${THEME_PREVIEW_VARS.background}, var(--color-base-200))` }}
      />
      <span
        className="h-2.5 w-2.5 rounded-full ring-1 ring-base-content/10"
        style={{ backgroundColor: `var(${THEME_PREVIEW_VARS.primary})` }}
      />
      <span
        className="h-2.5 w-2.5 rounded-full ring-1 ring-base-content/10"
        style={{ backgroundColor: `var(${THEME_PREVIEW_VARS.accent})` }}
      />
    </span>
  );
}
