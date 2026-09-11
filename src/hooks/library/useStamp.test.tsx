import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EMPTY_STAMP_CONFIG } from "../../lib/stamp";
import { useStamp } from "./useStamp";
import type { Album, TagDto } from "../../types";

const tags: TagDto[] = [
  { id: 1, name: "Travel", parent_id: null, color: "#f00", asset_count: 0 },
  { id: 2, name: "Work", parent_id: null, color: "#0f0", asset_count: 0 },
];

const albums: Album[] = [
  { id: 10, name: "Best", sort_mode: "date:desc", emoji: null, asset_count: 0 },
];

describe("useStamp", () => {
  it("auto arms when config becomes valid and disarms when invalid", () => {
    const { result } = renderHook(() => useStamp(tags, albums));

    expect(result.current.armed).toBe(false);

    act(() => {
      result.current.setStampRating(4);
    });
    expect(result.current.armed).toBe(true);

    act(() => {
      result.current.setStampRating(null);
      result.current.toggleStampTag(1, false);
      result.current.toggleStampAlbum(10, false);
    });
    expect(result.current.armed).toBe(false);
  });

  it("prunes deleted tag and album references", () => {
    const { result, rerender } = renderHook(
      ({ nextTags, nextAlbums }) => useStamp(nextTags, nextAlbums),
      {
        initialProps: { nextTags: tags, nextAlbums: albums },
      },
    );

    act(() => {
      result.current.toggleStampTag(1, true);
      result.current.toggleStampAlbum(10, true);
    });

    rerender({ nextTags: [tags[1]], nextAlbums: [] });
    expect(result.current.config.tag_ids).toEqual([]);
    expect(result.current.config.album_ids).toEqual([]);
  });

  it("resets config when disarmed", () => {
    const { result } = renderHook(() => useStamp(tags, albums));

    act(() => {
      result.current.setStampRating(4);
      result.current.toggleStampTag(1, true);
      result.current.toggleStampAlbum(10, true);
      result.current.markStampResults([1], []);
    });
    expect(result.current.armed).toBe(true);

    act(() => {
      result.current.disarm();
    });

    expect(result.current.armed).toBe(false);
    expect(result.current.config).toEqual(EMPTY_STAMP_CONFIG);
    expect(result.current.matchedIds.size).toBe(0);
  });

  it("removes tag and album ids when toggled off", () => {
    const { result } = renderHook(() => useStamp(tags, albums));

    act(() => {
      result.current.toggleStampTag(1, true);
      result.current.toggleStampAlbum(10, true);
      result.current.toggleStampTag(1, false);
      result.current.toggleStampAlbum(10, false);
    });

    expect(result.current.config.tag_ids).toEqual([]);
    expect(result.current.config.album_ids).toEqual([]);
  });

  it("ignores duplicate tag and album toggles", () => {
    const { result } = renderHook(() => useStamp(tags, albums));

    act(() => {
      result.current.toggleStampTag(1, true);
      result.current.toggleStampTag(1, true);
      result.current.toggleStampAlbum(10, true);
      result.current.toggleStampAlbum(10, true);
    });

    expect(result.current.config.tag_ids).toEqual([1]);
    expect(result.current.config.album_ids).toEqual([10]);
  });

  it("clears matched ids", () => {
    const { result } = renderHook(() => useStamp(tags, albums));

    act(() => {
      result.current.markStampResults([1], []);
      result.current.clearMatchedIds();
    });

    expect(result.current.matchedIds.size).toBe(0);
  });

  it("updates matched ids after stamp results", () => {
    const { result } = renderHook(() => useStamp(tags, albums));

    act(() => {
      result.current.markStampResults([1, 2], [3]);
    });

    expect(result.current.matchedIds.has(1)).toBe(true);
    expect(result.current.matchedIds.has(2)).toBe(true);
    expect(result.current.matchedIds.has(3)).toBe(false);
  });
});
