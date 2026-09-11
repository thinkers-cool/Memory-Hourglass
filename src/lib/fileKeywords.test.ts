import { describe, expect, it } from "vitest";
import { fileKeywordsNotInCatalog, parseKeywordsJson } from "./fileKeywords";

describe("fileKeywords", () => {
  it("parses keyword json arrays", () => {
    expect(parseKeywordsJson('["travel", "family"]')).toEqual(["travel", "family"]);
    expect(parseKeywordsJson(null)).toEqual([]);
    expect(parseKeywordsJson("")).toEqual([]);
    expect(parseKeywordsJson("not-json")).toEqual([]);
    expect(parseKeywordsJson('{"travel": true}')).toEqual([]);
  });

  it("returns keywords not present in the tag catalog", () => {
    expect(
      fileKeywordsNotInCatalog('["travel", "orphan"]', ["travel", "family"]),
    ).toEqual(["orphan"]);
  });
});
