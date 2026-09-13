import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen,
}));

import * as client from "./client";

describe("api client", () => {
  beforeEach(() => {
    invoke.mockReset();
    listen.mockReset();
    listen.mockResolvedValue(() => undefined);
    invoke.mockResolvedValue(undefined);
  });

  it("invokes workspace commands", async () => {
    await client.createWorkspace("/tmp/ws", true);
    await client.openWorkspace("/tmp/ws");
    await client.closeWorkspace();
    await client.getActiveWorkspace();
    await client.listRecentWorkspaces();
    await client.removeRecentWorkspace("/tmp/ws");
    await client.tryOpenLastWorkspace();

    expect(invoke).toHaveBeenCalledWith("create_workspace", {
      path: "/tmp/ws",
      readOnly: true,
    });
    expect(invoke).toHaveBeenCalledWith("open_workspace", { path: "/tmp/ws" });
    expect(invoke).toHaveBeenCalledWith("close_workspace");
    expect(invoke).toHaveBeenCalledWith("get_active_workspace");
    expect(invoke).toHaveBeenCalledWith("list_recent_workspaces");
    expect(invoke).toHaveBeenCalledWith("remove_recent_workspace", {
      path: "/tmp/ws",
    });
    expect(invoke).toHaveBeenCalledWith("try_open_last_workspace");
  });

  it("invokes smb browse commands", async () => {
    await client.listSmbShares({
      host: "nas",
      username: "user",
      password: "secret",
    });
    await client.mountSmbForBrowse({
      host: "nas",
      share: "photos",
      username: "user",
      password: "secret",
      domain: "WORKGROUP",
    });
    await client.listFolderChildren("/smb/photos");

    expect(invoke).toHaveBeenCalledWith("list_smb_shares", {
      request: { host: "nas", username: "user", password: "secret" },
    });
    expect(invoke).toHaveBeenCalledWith("mount_smb_for_browse", {
      request: {
        host: "nas",
        share: "photos",
        username: "user",
        password: "secret",
        domain: "WORKGROUP",
      },
    });
    expect(invoke).toHaveBeenCalledWith("list_folder_children", {
      path: "/smb/photos",
    });
  });

  it("invokes library commands", async () => {
    await client.addRoot("/photos");
    await client.addSmbSource({
      mode: "mounted",
      path: "/smb",
      poll_secs: 120,
    });
    await client.connectSmbShare({
      host: "nas",
      share: "photos",
      username: "user",
      password: "secret",
      pollSecs: 60,
      subPath: "media",
    });
    await client.relinkRoot(1, "/new");
    await client.previewRelink(1, "/new");
    await client.listRoots();
    await client.listRootStats();
    await client.removeRoot(2);

    expect(invoke).toHaveBeenCalledWith("add_root", { path: "/photos" });
    expect(invoke).toHaveBeenCalledWith("add_smb_source", {
      input: {
        mode: "mounted",
        path: "/smb",
        poll_secs: 120,
      },
    });
    expect(invoke).toHaveBeenCalledWith("add_smb_source", {
      input: {
        mode: "connect",
        host: "nas",
        share: "photos",
        username: "user",
        password: "secret",
        poll_secs: 60,
        sub_path: "media",
      },
    });
    expect(invoke).toHaveBeenCalledWith("relink_root", { id: 1, path: "/new" });
    expect(invoke).toHaveBeenCalledWith("preview_relink", {
      id: 1,
      path: "/new",
    });
    expect(invoke).toHaveBeenCalledWith("list_roots");
    expect(invoke).toHaveBeenCalledWith("list_root_stats");
    expect(invoke).toHaveBeenCalledWith("remove_root", { id: 2 });
  });

  it("invokes scan commands", async () => {
    await client.startScan(3);
    await client.cancelScan();
    await client.pauseScan();
    await client.resumeScan();
    await client.rebuildCatalog();
    await client.getScanStatus(3);
    await client.listScanStatuses();
    await client.resumePendingScans();

    expect(invoke).toHaveBeenCalledWith("start_scan", { rootId: 3 });
    expect(invoke).toHaveBeenCalledWith("cancel_scan", { rootId: null });
    expect(invoke).toHaveBeenCalledWith("pause_scan");
    expect(invoke).toHaveBeenCalledWith("resume_scan");
    expect(invoke).toHaveBeenCalledWith("rebuild_catalog");
    expect(invoke).toHaveBeenCalledWith("get_scan_status", { rootId: 3 });
    expect(invoke).toHaveBeenCalledWith("list_scan_statuses");
    expect(invoke).toHaveBeenCalledWith("resume_pending_scans");
  });

  it("invokes query commands", async () => {
    invoke.mockResolvedValueOnce({ total: 0, items: [] });
    await client.queryAssets({ root_id: 2 }, "name:asc", 50, 100);
    await client.countAssets({ root_id: 2 });

    expect(invoke).toHaveBeenCalledWith("query_assets", {
      filter: { root_id: 2 },
      sort: "name:asc",
      offset: 50,
      limit: 100,
    });
    expect(invoke).toHaveBeenCalledWith("count_assets", {
      filter: { root_id: 2 },
    });
  });

  it("invokes asset commands", async () => {
    await client.getAsset(1);
    await client.updateAssetMeta(1, { rating: 4 });
    await client.batchAppendTags([1, 2], 3);
    await client.batchRemoveTags([1], 5);
    await client.batchUpdateAssetMeta([1], { rating: 3 });
    await client.softDeleteAssets([1, 2]);
    await client.restoreAssets([1]);
    await client.purgeDelete([1], "DELETE");

    expect(invoke).toHaveBeenCalledWith("get_asset", { id: 1 });
    expect(invoke).toHaveBeenCalledWith("update_asset_meta", {
      id: 1,
      patch: { rating: 4 },
    });
    expect(invoke).toHaveBeenCalledWith("batch_append_tags", {
      assetIds: [1, 2],
      tagId: 3,
    });
    expect(invoke).toHaveBeenCalledWith("batch_remove_tags", {
      assetIds: [1],
      tagId: 5,
    });
    expect(invoke).toHaveBeenCalledWith("batch_update_asset_meta", {
      ids: [1],
      patch: { rating: 3 },
    });
    expect(invoke).toHaveBeenCalledWith("soft_delete_assets", { ids: [1, 2] });
    expect(invoke).toHaveBeenCalledWith("restore_assets", { ids: [1] });
    expect(invoke).toHaveBeenCalledWith("purge_delete", {
      ids: [1],
      confirmToken: "DELETE",
    });
  });

  it("invokes tag and collection commands", async () => {
    await client.listTags();
    await client.createTag("travel", 1, "#fff");
    await client.updateTag(2, "renamed", "#000");
    await client.deleteTag(5);
    await client.listSmartCollections();
    await client.saveSmartCollection("Favorites", { rating_min: 4 });
    await client.deleteSmartCollection(9);
    await client.listAlbums();
    await client.createAlbum("Summer", "date:desc", "☀️");
    await client.updateAlbum(4, "Renamed", "🌙");
    await client.deleteAlbum(3);
    await client.setAlbumItems(3, [1, 2]);
    await client.addAlbumItems(3, [4]);
    await client.removeAlbumItems(3, [4]);

    expect(invoke).toHaveBeenCalledWith("list_tags");
    expect(invoke).toHaveBeenCalledWith("create_tag", {
      name: "travel",
      parentId: 1,
      color: "#fff",
    });
    expect(invoke).toHaveBeenCalledWith("update_tag", {
      id: 2,
      name: "renamed",
      color: "#000",
    });
    expect(invoke).toHaveBeenCalledWith("delete_tag", { id: 5 });
    expect(invoke).toHaveBeenCalledWith("list_smart_collections");
    expect(invoke).toHaveBeenCalledWith("save_smart_collection", {
      name: "Favorites",
      filter: { rating_min: 4 },
    });
    expect(invoke).toHaveBeenCalledWith("delete_smart_collection", { id: 9 });
    expect(invoke).toHaveBeenCalledWith("list_albums");
    expect(invoke).toHaveBeenCalledWith("create_album", {
      name: "Summer",
      sortMode: "date:desc",
      emoji: "☀️",
    });
    expect(invoke).toHaveBeenCalledWith("update_album", {
      id: 4,
      name: "Renamed",
      emoji: "🌙",
    });
    expect(invoke).toHaveBeenCalledWith("delete_album", { id: 3 });
    expect(invoke).toHaveBeenCalledWith("set_album_items", {
      albumId: 3,
      assetIds: [1, 2],
    });
    expect(invoke).toHaveBeenCalledWith("add_album_items", {
      albumId: 3,
      assetIds: [4],
    });
    expect(invoke).toHaveBeenCalledWith("remove_album_items", {
      albumId: 3,
      assetIds: [4],
    });
  });

  it("invokes export commands", async () => {
    vi.spyOn(Date, "now").mockReturnValue(1234);
    await client.startExport([1], "/out", { flat: true });
    await client.startExport([2], "/out2", { flat: false }, 99);
    await client.cancelExport();
    await client.getExportStatus();
    await client.listExportJobs();

    expect(invoke).toHaveBeenCalledWith("start_export", {
      assetIds: [1],
      destination: "/out",
      options: { flat: true },
      jobId: 1234,
    });
    expect(invoke).toHaveBeenCalledWith("start_export", {
      assetIds: [2],
      destination: "/out2",
      options: { flat: false },
      jobId: 99,
    });
    expect(invoke).toHaveBeenCalledWith("cancel_export");
    expect(invoke).toHaveBeenCalledWith("get_export_status");
    expect(invoke).toHaveBeenCalledWith("list_export_jobs");
  });

  it("invokes album asset lookup", async () => {
    await client.getAlbumAssetIds(12);
    expect(invoke).toHaveBeenCalledWith("get_album_asset_ids", { albumId: 12 });
  });

  it("invokes activity commands", async () => {
    await client.queryAssetActivity(42, 10, 25);
    await client.undoActivity(7);

    expect(invoke).toHaveBeenCalledWith("query_asset_activity", {
      assetId: 42,
      offset: 10,
      limit: 25,
    });
    expect(invoke).toHaveBeenCalledWith("undo_activity", { activityId: 7 });
  });

  it("invokes scan status with optional root id", async () => {
    await client.getScanStatus();
    await client.getScanStatus(4);
    expect(invoke).toHaveBeenCalledWith("get_scan_status", { rootId: null });
    expect(invoke).toHaveBeenCalledWith("get_scan_status", { rootId: 4 });
  });

  it("registers scan thumb listener", async () => {
    const handler = vi.fn();
    await client.onScanThumbs(handler);
    expect(listen).toHaveBeenCalledWith("scan://thumbs", expect.any(Function));

    const thumbCallback = listen.mock.calls.find(
      ([channel]) => channel === "scan://thumbs",
    )?.[1];
    thumbCallback?.({
      payload: {
        root_id: 1,
        thumbs: [{ asset_id: 2, thumb_path: "/tmp/2.webp" }],
      },
    });
    expect(handler).toHaveBeenCalledWith({
      root_id: 1,
      thumbs: [{ asset_id: 2, thumb_path: "/tmp/2.webp" }],
    });
  });

  it("registers progress listeners", async () => {
    const scanHandler = vi.fn();
    const jobHandler = vi.fn();
    await client.onScanProgress(scanHandler);
    await client.onJobProgress(jobHandler);
    expect(listen).toHaveBeenCalledWith(
      "scan://progress",
      expect.any(Function),
    );
    expect(listen).toHaveBeenCalledWith("job://progress", expect.any(Function));

    const scanCallback = listen.mock.calls.find(
      ([channel]) => channel === "scan://progress",
    )?.[1];
    const jobCallback = listen.mock.calls.find(
      ([channel]) => channel === "job://progress",
    )?.[1];
    scanCallback?.({
      payload: { root_id: 1, stage: "scanning", scanned: 1, indexed: 1 },
    });
    jobCallback?.({
      payload: { job_id: 1, phase: "started", done: 0, total: 1 },
    });
    expect(scanHandler).toHaveBeenCalledWith({
      root_id: 1,
      stage: "scanning",
      scanned: 1,
      indexed: 1,
    });
    expect(jobHandler).toHaveBeenCalledWith({
      job_id: 1,
      phase: "started",
      done: 0,
      total: 1,
    });
  });

  it("registers message notify listener", async () => {
    const handler = vi.fn();
    await client.onMessageNotify(handler);
    expect(listen).toHaveBeenCalledWith(
      "message://notify",
      expect.any(Function),
    );

    const notifyCallback = listen.mock.calls.find(
      ([channel]) => channel === "message://notify",
    )?.[1];
    const envelope = {
      kind: "success",
      source: "library",
      text_key: "library:notification.saved",
      text_params: {},
      actions: [],
    };
    notifyCallback?.({ payload: envelope });
    expect(handler).toHaveBeenCalledWith(envelope);
  });
});
