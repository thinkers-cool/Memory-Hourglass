export function indicesInWindow(index: number, length: number, window: number): number[] {
  const set = new Set<number>([index]);
  for (let offset = 1; offset <= window; offset++) {
    if (index - offset >= 0) set.add(index - offset);
    if (index + offset < length) set.add(index + offset);
  }
  return [...set];
}
