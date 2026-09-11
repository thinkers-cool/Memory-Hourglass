export const GRID_CARD_DOUBLE_CLICK_WINDOW_MS = 400;

let lastClick: { cardId: number; time: number } | null = null;

export function resetGridCardClickState() {
  lastClick = null;
}

export function handleGridCardClick(
  cardId: number,
  onSingleClick: () => void,
  onDoubleClick: () => void,
  windowMs = GRID_CARD_DOUBLE_CLICK_WINDOW_MS,
) {
  const now = Date.now();

  if (lastClick?.cardId === cardId && now - lastClick.time < windowMs) {
    lastClick = null;
    onDoubleClick();
    return;
  }

  lastClick = { cardId, time: now };
  onSingleClick();
}
