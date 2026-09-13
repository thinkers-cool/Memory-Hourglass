import { describe, expect, it } from "vitest";
import { pickActiveScanStatus } from "./scanStatusSelection";

describe("pickActiveScanStatus", () => {
  it("returns the first active scan status", () => {
    expect(
      pickActiveScanStatus({
        1: "done: 1/1",
        2: "indexing: 2/4",
      }),
    ).toBe("indexing: 2/4");
  });

  it("returns empty when no scans are active", () => {
    expect(pickActiveScanStatus({ 1: "done: 1/1" })).toBe("");
  });
});
