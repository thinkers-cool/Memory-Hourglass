export type AnchoredPopoverPlacement = {
  top: number;
  left: number;
  width: number;
  height: number;
};

export type AnchoredPopoverBounds = {
  anchorLeft: number;
  anchorTop: number;
  anchorBottom: number;
  boundaryLeft: number;
  boundaryRight: number;
  boundaryTop: number;
  boundaryBottom: number;
  preferredWidth: number;
  preferredHeight: number;
  margin: number;
};

export function computeAnchoredPopoverPlacement(
  bounds: AnchoredPopoverBounds,
): AnchoredPopoverPlacement {
  const {
    anchorLeft,
    anchorTop,
    anchorBottom,
    boundaryLeft,
    boundaryRight,
    boundaryTop,
    boundaryBottom,
    preferredWidth,
    preferredHeight,
    margin,
  } = bounds;

  const width = Math.max(
    168,
    Math.min(preferredWidth, boundaryRight - boundaryLeft - margin * 2),
  );

  let left = anchorLeft;
  left = Math.max(boundaryLeft + margin, left);
  left = Math.min(left, boundaryRight - width - margin);

  const spaceBelow = boundaryBottom - anchorBottom - margin;
  const spaceAbove = anchorTop - boundaryTop - margin;
  const openAbove =
    spaceBelow < preferredHeight * 0.75 && spaceAbove > spaceBelow;

  const height = Math.max(
    180,
    Math.min(
      preferredHeight,
      openAbove ? spaceAbove - margin : spaceBelow - margin,
    ),
  );

  const top = openAbove
    ? Math.max(boundaryTop + margin, anchorTop - height - margin)
    : Math.min(boundaryBottom - height - margin, anchorBottom + margin);

  return { top, left, width, height };
}

export type FloatingMenuPlacement = "top-end" | "bottom-end";

export function computeFloatingMenuPlacement(
  anchor: Pick<DOMRect, "top" | "right" | "bottom">,
  menu: { width: number; height: number },
  placement: FloatingMenuPlacement,
  margin = 8,
  viewport = { width: window.innerWidth, height: window.innerHeight },
): { top: number; left: number } {
  let left = anchor.right - menu.width;
  left = Math.max(margin, Math.min(left, viewport.width - menu.width - margin));

  let top =
    placement === "top-end"
      ? anchor.top - menu.height - margin
      : anchor.bottom + margin;
  top = Math.max(margin, Math.min(top, viewport.height - menu.height - margin));

  return { top, left };
}
