import { describe, expect, it, vi } from "vitest";
import type { Album, RootStats, TagDto } from "../types";
import {
  DEFAULT_PANEL_ORDER_STORE,
  loadPanelOrderStore,
  orderAlbums,
  orderRoots,
  orderTagsForParent,
  panelOrderStorageKey,
  parsePanelOrderStore,
  savePanelOrderStore,
  togglePanelOrderMode,
} from "./panelOrder";

const roots: RootStats[] = [
  {
    id: 2,
    path: "/photos/B",
    kind: "local",
    status: "idle",
    scan_policy: "watch",
    poll_secs: null,
    last_scan_at: null,
    asset_count: 1,
    missing_count: 0,
  },
  {
    id: 1,
    path: "/photos/A",
    kind: "local",
    status: "idle",
    scan_policy: "watch",
    poll_secs: null,
    last_scan_at: null,
    asset_count: 1,
    missing_count: 0,
  },
];

const albums: Album[] = [
  {
    id: 2,
    name: "Beta",
    sort_mode: "date:desc",
    emoji: null,
    asset_count: 1,
  },
  {
    id: 1,
    name: "Alpha",
    sort_mode: "date:desc",
    emoji: null,
    asset_count: 1,
  },
];

describe("panelOrder", () => {
  it("sorts by name ascending and descending", () => {
    expect(
      orderRoots(
        [{ ...roots[0], path: "/" }, ...roots],
        "name-asc",
      ).map((root) => root.path),
    ).toEqual(["/", "/photos/A", "/photos/B"]);
    expect(orderRoots(roots, "name-asc").map((root) => root.id)).toEqual([
      1, 2,
    ]);
    expect(orderAlbums(albums, "name-desc").map((album) => album.id)).toEqual([
      2, 1,
    ]);
  });

  it("toggles between ascending and descending", () => {
    expect(togglePanelOrderMode("name-asc")).toBe("name-desc");
    expect(togglePanelOrderMode("name-desc")).toBe("name-asc");
  });

  it("parses stored panel order modes", () => {
    expect(parsePanelOrderStore(null)).toBeNull();
    expect(parsePanelOrderStore({ roots: "name-desc" })).toEqual({
      roots: "name-desc",
      albums: "name-asc",
      tags: "name-asc",
    });
    expect(
      parsePanelOrderStore({ albums: { mode: "name-desc" } }),
    ).toEqual({
      roots: "name-asc",
      albums: "name-desc",
      tags: "name-asc",
    });
    expect(
      parsePanelOrderStore({ roots: { mode: "name-desc" } }),
    ).toEqual({
      roots: "name-desc",
      albums: "name-asc",
      tags: "name-asc",
    });
    expect(parsePanelOrderStore({ tags: { mode: "invalid" } })).toEqual({
      roots: "name-asc",
      albums: "name-asc",
      tags: "name-asc",
    });
  });

  it("ignores storage write failures", () => {
    const setItem = vi
      .spyOn(Storage.prototype, "setItem")
      .mockImplementation(() => {
        throw new Error("quota");
      });
    expect(() =>
      savePanelOrderStore("ws-save-fail", DEFAULT_PANEL_ORDER_STORE),
    ).not.toThrow();
    setItem.mockRestore();
    savePanelOrderStore("ws-save-fail", DEFAULT_PANEL_ORDER_STORE);
  });

  it("loads and saves panel order per workspace", () => {
    const key = panelOrderStorageKey("ws-1");
    localStorage.setItem(
      key,
      JSON.stringify({ roots: "name-desc", albums: "name-desc", tags: "name-desc" }),
    );
    expect(loadPanelOrderStore("ws-1").roots).toBe("name-desc");
    savePanelOrderStore("ws-2", DEFAULT_PANEL_ORDER_STORE);
    expect(localStorage.getItem(panelOrderStorageKey("ws-2"))).toContain(
      "name-asc",
    );
  });

  it("orders tags per parent group", () => {
    const tags: TagDto[] = [
      { id: 2, name: "zulu", parent_id: null, color: null, asset_count: 1 },
      { id: 1, name: "alpha", parent_id: null, color: null, asset_count: 1 },
      { id: 4, name: "child-b", parent_id: 1, color: null, asset_count: 0 },
      { id: 3, name: "child-a", parent_id: 1, color: null, asset_count: 0 },
    ];
    expect(
      orderTagsForParent(tags, 1, "name-asc").map((tag) => tag.id),
    ).toEqual([3, 4]);
  });
});
