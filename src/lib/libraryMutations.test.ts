import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api/client", () => ({
  batchAppendTags: vi.fn(),
  batchRemoveTags: vi.fn(),
  createTag: vi.fn(),
  createAlbum: vi.fn(),
  addAlbumItems: vi.fn(),
  removeAlbumItems: vi.fn(),
}));

import * as api from "../api/client";
import {
  runAlbumCreate,
  runAlbumToggle,
  runTagCreate,
  runTagToggle,
} from "./libraryMutations";

describe("libraryMutations", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("toggles tags on and off", async () => {
    vi.mocked(api.batchAppendTags).mockResolvedValue(2);
    vi.mocked(api.batchRemoveTags).mockResolvedValue(1);
    await expect(runTagToggle([1], 3, true)).resolves.toBe(2);
    await expect(runTagToggle([1], 3, false)).resolves.toBe(1);
  });

  it("creates tag and assigns assets", async () => {
    vi.mocked(api.createTag).mockResolvedValue({ id: 9, name: "trip", parent_id: null, color: null });
    vi.mocked(api.batchAppendTags).mockResolvedValue(1);
    await expect(runTagCreate([1, 2], " trip ")).resolves.toBe(9);
    expect(api.createTag).toHaveBeenCalledWith("trip");
  });

  it("returns zero for empty tag name", async () => {
    await expect(runTagCreate([1], "   ")).resolves.toBe(0);
    expect(api.createTag).not.toHaveBeenCalled();
  });

  it("toggles albums on and off", async () => {
    vi.mocked(api.addAlbumItems).mockResolvedValue(2);
    vi.mocked(api.removeAlbumItems).mockResolvedValue(1);
    await expect(runAlbumToggle([1], 4, true)).resolves.toBe(2);
    await expect(runAlbumToggle([1], 4, false)).resolves.toBe(1);
  });

  it("creates album and assigns assets", async () => {
    vi.mocked(api.createAlbum).mockResolvedValue({ id: 5, name: "set", emoji: null, is_smart: false });
    vi.mocked(api.addAlbumItems).mockResolvedValue(1);
    await expect(runAlbumCreate([1], " set ")).resolves.toBe(5);
  });

  it("returns zero for empty album name", async () => {
    await expect(runAlbumCreate([1], "")).resolves.toBe(0);
  });
});
