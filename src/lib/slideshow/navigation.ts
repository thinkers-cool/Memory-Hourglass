import type { AssetCard } from "../../types";

export function isPlayable(card: AssetCard): boolean {
  return card.sync_state !== "missing";
}

export function playableIndices(items: AssetCard[]): number[] {
  const indices: number[] = [];
  for (let i = 0; i < items.length; i++) {
    if (isPlayable(items[i])) indices.push(i);
  }
  return indices;
}

export function buildShuffleOrder(items: AssetCard[], startAt: number): number[] {
  const order = playableIndices(items);
  for (let i = order.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [order[i], order[j]] = [order[j], order[i]];
  }
  const startPos = order.indexOf(startAt);
  if (startPos > 0) {
    [order[0], order[startPos]] = [order[startPos], order[0]];
  }
  return order;
}

export function resolveNextIndex(
  items: AssetCard[],
  current: number,
  delta: number,
  loop: boolean,
): number | null {
  const playable = playableIndices(items);
  if (playable.length === 0) return null;
  const pos = playable.indexOf(current);
  const anchor = pos >= 0 ? pos : 0;
  const nextPos = anchor + delta;
  if (nextPos >= 0 && nextPos < playable.length) return playable[nextPos];
  if (!loop || playable.length === 1) return null;
  return delta > 0 ? playable[0] : playable[playable.length - 1];
}

export function resolveShuffleIndex(
  order: number[],
  current: number,
  delta: number,
  loop: boolean,
): number | null {
  if (order.length === 0) return null;
  const pos = order.indexOf(current);
  const anchor = pos >= 0 ? pos : 0;
  const nextPos = anchor + delta;
  if (nextPos >= 0 && nextPos < order.length) return order[nextPos];
  if (!loop || order.length === 1) return null;
  return delta > 0 ? order[0] : order[order.length - 1];
}
