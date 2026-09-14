export const GRID_CARD_DOUBLE_CLICK_WINDOW_MS = 400;

let lastClick: { cardId: number; time: number } | null = null;
let pendingSingleClick: {
  timer: ReturnType<typeof setTimeout>;
  onSingleClick: () => void;
} | null = null;

function flushPendingSingleClick() {
  if (!pendingSingleClick) return;
  clearTimeout(pendingSingleClick.timer);
  const onSingleClick = pendingSingleClick.onSingleClick;
  pendingSingleClick = null;
  onSingleClick();
}

function clearPendingSingleClick() {
  if (!pendingSingleClick) return;
  clearTimeout(pendingSingleClick.timer);
  pendingSingleClick = null;
}

export function resetGridCardClickState() {
  lastClick = null;
  clearPendingSingleClick();
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
    clearPendingSingleClick();
    onDoubleClick();
    return;
  }

  flushPendingSingleClick();
  lastClick = { cardId, time: now };
  pendingSingleClick = {
    onSingleClick,
    timer: setTimeout(() => {
      pendingSingleClick = null;
      lastClick = null;
      onSingleClick();
    }, windowMs),
  };
}
