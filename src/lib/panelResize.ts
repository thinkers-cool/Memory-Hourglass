export const PANEL_DIVIDER_WIDTH_PX = 8;

export function clampLeadingPanelWidth(
  nextWidth: number,
  containerWidth: number,
  minLeading: number,
  minTrailing: number,
  dividerWidth = PANEL_DIVIDER_WIDTH_PX,
): number {
  const maxLeading = containerWidth - minTrailing - dividerWidth;
  return Math.max(minLeading, Math.min(maxLeading, nextWidth));
}

export function clampTrailingPanelWidth(
  nextWidth: number,
  containerWidth: number,
  minMain: number,
  minSide: number,
  dividerWidth = PANEL_DIVIDER_WIDTH_PX,
): number {
  const maxSide = containerWidth - minMain - dividerWidth;
  return Math.max(minSide, Math.min(maxSide, nextWidth));
}
