import type { SortMode, SortDir } from "../types";

export type { SortDir };

export const DEFAULT_SORT_DIR: Record<SortMode, SortDir> = {
  date: "desc",
  name: "asc",
  rating: "desc",
  path: "asc",
};

export function encodeSortParam(sort: SortMode, dir: SortDir): string {
  return `${sort}:${dir}`;
}
