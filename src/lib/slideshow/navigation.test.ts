import { describe, expect, it } from "vitest";
import type { AssetCard } from "../../types";
import {
  buildShuffleOrder,
  isPlayable,
  playableIndices,
  resolveNextIndex,
  resolveShuffleIndex,
} from "./navigation";

function card(id: number, sync_state = "ok"): AssetCard {
  return {
    id,
    file_name: `file-${id}.jpg`,
    ext: "jpg",
    kind: "image",
    capture_at: null,
    rating: null,
    sync_state,
    thumb_path: null,
    abs_path: `/tmp/${id}.jpg`,
    has_duplicate: false,
  };
}

describe("isPlayable", () => {
  it("rejects missing assets", () => {
    expect(isPlayable(card(1, "missing"))).toBe(false);
    expect(isPlayable(card(1, "ok"))).toBe(true);
  });
});

describe("playableIndices", () => {
  it("returns only playable item indices", () => {
    const items = [card(1), card(2, "missing"), card(3)];
    expect(playableIndices(items)).toEqual([0, 2]);
  });
});

describe("resolveNextIndex", () => {
  const items = [card(1), card(2, "missing"), card(3)];

  it("skips missing items forward", () => {
    expect(resolveNextIndex(items, 0, 1, false)).toBe(2);
  });

  it("wraps when loop is enabled", () => {
    expect(resolveNextIndex(items, 2, 1, true)).toBe(0);
    expect(resolveNextIndex(items, 0, -1, true)).toBe(2);
  });

  it("stops at end without loop", () => {
    expect(resolveNextIndex(items, 2, 1, false)).toBeNull();
    expect(resolveNextIndex(items, 0, -1, false)).toBeNull();
  });

  it("returns null when no playable items exist", () => {
    expect(
      resolveNextIndex([card(1, "missing")], 0, 1, true),
    ).toBeNull();
  });

  it("anchors to first playable item when current is missing", () => {
    expect(resolveNextIndex(items, 1, 1, false)).toBe(2);
  });

  it("does not loop single-item playlists", () => {
    expect(resolveNextIndex([card(1)], 0, 1, true)).toBeNull();
  });
});

describe("buildShuffleOrder", () => {
  it("places start index first", () => {
    const items = [card(1), card(2), card(3), card(4), card(5)];
    const order = buildShuffleOrder(items, 3);
    expect(order[0]).toBe(3);
    expect(order).toHaveLength(5);
  });

  it("omits missing assets", () => {
    const items = [card(1), card(2, "missing"), card(3)];
    const order = buildShuffleOrder(items, 1);
    expect(order).not.toContain(1);
    expect(order).toHaveLength(2);
  });

  it("keeps start index first when already at front", () => {
    const items = [card(1), card(2), card(3)];
    const order = buildShuffleOrder(items, 0);
    expect(order[0]).toBe(0);
  });
});

describe("resolveShuffleIndex", () => {
  const order = [2, 0, 1];

  it("follows shuffle order", () => {
    expect(resolveShuffleIndex(order, 2, 1, false)).toBe(0);
  });

  it("wraps with loop", () => {
    expect(resolveShuffleIndex(order, 1, 1, true)).toBe(2);
    expect(resolveShuffleIndex(order, 2, -1, true)).toBe(1);
  });

  it("returns null at boundaries without loop", () => {
    expect(resolveShuffleIndex(order, 1, 1, false)).toBeNull();
    expect(resolveShuffleIndex(order, 2, -1, false)).toBeNull();
  });

  it("returns null for empty order", () => {
    expect(resolveShuffleIndex([], 0, 1, true)).toBeNull();
  });

  it("anchors to first item when current is absent", () => {
    expect(resolveShuffleIndex(order, 99, 1, false)).toBe(0);
  });

  it("does not loop single-item orders", () => {
    expect(resolveShuffleIndex([5], 5, 1, true)).toBeNull();
  });
});
