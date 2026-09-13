import type { CSSProperties } from "react";

export function normalizeRotation(degrees: number): number {
  const normalized = degrees % 360;
  return normalized < 0 ? normalized + 360 : normalized;
}

export function rotateClockwise(degrees: number): number {
  return normalizeRotation(degrees + 90);
}

export function rotateCounterClockwise(degrees: number): number {
  return normalizeRotation(degrees - 90);
}

export function resolveRotation(rotation?: number | null): number {
  return normalizeRotation(rotation ?? 0);
}

export function imageRotationStyle(rotation: number): CSSProperties {
  const degrees = resolveRotation(rotation);
  if (degrees === 0) {
    return {};
  }
  return { transform: `rotate(${degrees}deg)` };
}
