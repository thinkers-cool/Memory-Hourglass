import { describe, expect, it } from "vitest";
import {
  parseScanProgress,
  resolveRootScanProgress,
  scanProgressCounts,
} from "./scanStatus";

describe("parseScanProgress", () => {
  it("parses scan phases with counts", () => {
    expect(parseScanProgress("scanning: 2/10")).toEqual({
      phase: "scanning",
      done: 2,
      total: 10,
    });
    expect(parseScanProgress("cataloging: 1/10")).toEqual({
      phase: "cataloging",
      done: 1,
      total: 10,
    });
    expect(parseScanProgress("indexing: 160/1205")).toEqual({
      phase: "indexing",
      done: 160,
      total: 1205,
    });
  });

  it("parses scan phases without counts", () => {
    expect(parseScanProgress("indexing:")).toEqual({
      phase: "indexing",
      done: null,
      total: null,
    });
  });

  it("returns null for unknown statuses", () => {
    expect(parseScanProgress("done: 10 files")).toBeNull();
    expect(parseScanProgress("queued: 1/10")).toBeNull();
  });
});

describe("scanProgressCounts", () => {
  it("formats done and total", () => {
    expect(
      scanProgressCounts({
        phase: "indexing",
        done: 160,
        total: 1205,
      }),
    ).toBe("160/1205");
  });

  it("returns empty string when counts are missing", () => {
    expect(
      scanProgressCounts({
        phase: "indexing",
        done: null,
        total: null,
      }),
    ).toBe("");
  });
});

describe("resolveRootScanProgress", () => {
  it("returns parsed progress for active root scans", () => {
    expect(resolveRootScanProgress("indexing: 1/2")).toEqual({
      phase: "indexing",
      done: 1,
      total: 2,
    });
  });

  it("returns null for finished or missing statuses", () => {
    expect(resolveRootScanProgress("done: 10 files")).toBeNull();
    expect(resolveRootScanProgress(undefined)).toBeNull();
  });
});
