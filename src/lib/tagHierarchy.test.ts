import { describe, expect, it } from "vitest";
import {
  collectDescendantTagIds,
  tagChildren,
  tagDepthFirst,
  tagPathLabel,
  tagRoots,
} from "./tagHierarchy";
import type { TagDto } from "../types";

const tags: TagDto[] = [
  { id: 1, name: "travel", parent_id: null, color: "#111111", asset_count: 2 },
  { id: 2, name: "japan", parent_id: 1, color: "#222222", asset_count: 1 },
  { id: 3, name: "tokyo", parent_id: 2, color: "#333333", asset_count: 1 },
  { id: 4, name: "food", parent_id: null, color: "#444444", asset_count: 0 },
];

describe("tagHierarchy", () => {
  it("orders tags depth-first", () => {
    expect(tagDepthFirst(tags).map((tag) => tag.id)).toEqual([4, 1, 2, 3]);
  });

  it("builds hierarchical labels", () => {
    expect(tagPathLabel(tags[2], tags)).toBe("travel / japan / tokyo");
  });

  it("collects descendant ids", () => {
    expect(collectDescendantTagIds(1, tags)).toEqual([1, 2, 3]);
  });

  it("lists children and roots", () => {
    expect(tagRoots(tags).map((tag) => tag.id)).toEqual([4, 1]);
    expect(tagChildren(tags, 1).map((tag) => tag.id)).toEqual([2]);
  });
});
