import { describe, expect, it } from "vitest";
import { filterRawTags } from "../lib/metadataSearch";
import type { RawTag } from "../types";

const tags: RawTag[] = [
  { name: "ISO", value: "400" },
  { name: "LensModel", value: "35mm" },
  { name: "ExposureTime", value: "1/250" },
];

describe("filterRawTags", () => {
  it("returns all tags for empty query", () => {
    expect(filterRawTags(tags, "")).toEqual(tags);
    expect(filterRawTags(tags, "   ")).toEqual(tags);
  });

  it("matches tag names case-insensitively", () => {
    expect(filterRawTags(tags, "lens")).toEqual([tags[1]]);
  });

  it("matches tag values case-insensitively", () => {
    expect(filterRawTags(tags, "1/250")).toEqual([tags[2]]);
  });

  it("returns empty array when nothing matches", () => {
    expect(filterRawTags(tags, "missing")).toEqual([]);
  });
});
