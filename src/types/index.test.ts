import { describe, expect, it, vi } from "vitest";
import { DEFAULT_EXPORT_OPTIONS } from "../lib/libraryActions";
import type { AssetCard, QueryResult, RootStats } from "./index";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: vi.fn(() => false),
}));

describe("frontend types", () => {
  it("models asset card shape", () => {
    const card: AssetCard = {
      id: 1,
      file_name: "test.jpg",
      ext: "jpg",
      kind: "image",
      capture_at: 1,
      rating: 4,
      sync_state: "ok",
      thumb_path: "/tmp/1.webp",
      abs_path: "/photos/test.jpg",
      has_duplicate: false,
    };
    expect(card.rating).toBe(4);
  });

  it("models query result", () => {
    const result: QueryResult = { total: 0, items: [] };
    expect(result.items).toHaveLength(0);
  });

  it("models root stats", () => {
    const stats: RootStats = {
      id: 1,
      path: "/photos",
      kind: "local",
      status: "idle",
      scan_policy: "watch",
      poll_secs: null,
      last_scan_at: 1,
      asset_count: 10,
      missing_count: 0,
    };
    expect(stats.asset_count).toBe(10);
  });
});

describe("libraryActions defaults", () => {
  it("defines export defaults", () => {
    expect(DEFAULT_EXPORT_OPTIONS).toEqual({
      flat: true,
      rename_template: undefined,
      format: undefined,
    });
  });
});
