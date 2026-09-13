import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActivityEntry,
  Album,
  AssetDetail,
  AssetFilter,
  ExportJobSummary,
  ExportOptions,
  ExportStatus,
  JobProgress,
  QueryResult,
  RecentWorkspace,
  RelinkPreview,
  RootStats,
  ScanProgress,
  ScanThumbsEvent,
  SmartCollection,
  FolderEntry,
  SmbConnectRequest,
  SmbListSharesRequest,
  SmbShareEntry,
  SmbSourceInput,
  SourceRoot,
  TagDto,
  WorkspaceInfo,
} from "../types";

export async function createWorkspace(
  path: string,
  readOnly = false,
): Promise<WorkspaceInfo> {
  return invoke<WorkspaceInfo>("create_workspace", { path, readOnly });
}

export async function openWorkspace(path: string): Promise<WorkspaceInfo> {
  return invoke<WorkspaceInfo>("open_workspace", { path });
}

export async function closeWorkspace(): Promise<void> {
  return invoke("close_workspace");
}

export async function getActiveWorkspace(): Promise<WorkspaceInfo | null> {
  return invoke<WorkspaceInfo | null>("get_active_workspace");
}

export async function listRecentWorkspaces(): Promise<RecentWorkspace[]> {
  return invoke<RecentWorkspace[]>("list_recent_workspaces");
}

export async function removeRecentWorkspace(path: string): Promise<void> {
  return invoke("remove_recent_workspace", { path });
}

export async function tryOpenLastWorkspace(): Promise<WorkspaceInfo | null> {
  return invoke<WorkspaceInfo | null>("try_open_last_workspace");
}

export async function addRoot(path: string): Promise<SourceRoot> {
  return invoke<SourceRoot>("add_root", { path });
}

export async function addSmbSource(input: SmbSourceInput): Promise<SourceRoot> {
  return invoke<SourceRoot>("add_smb_source", { input });
}

export async function listSmbShares(
  request: SmbListSharesRequest,
): Promise<SmbShareEntry[]> {
  return invoke<SmbShareEntry[]>("list_smb_shares", { request });
}

export async function mountSmbForBrowse(
  request: SmbConnectRequest,
): Promise<string> {
  return invoke<string>("mount_smb_for_browse", {
    request: {
      host: request.host,
      share: request.share,
      username: request.username,
      password: request.password,
      domain: request.domain,
    },
  });
}

export async function listFolderChildren(path: string): Promise<FolderEntry[]> {
  return invoke<FolderEntry[]>("list_folder_children", { path });
}

export async function connectSmbShare(
  request: SmbConnectRequest,
): Promise<SourceRoot> {
  return addSmbSource({
    mode: "connect",
    host: request.host,
    share: request.share,
    username: request.username,
    password: request.password,
    domain: request.domain,
    poll_secs: request.pollSecs,
    sub_path: request.subPath,
  });
}

export async function relinkRoot(
  id: number,
  path: string,
): Promise<SourceRoot> {
  return invoke<SourceRoot>("relink_root", { id, path });
}

export async function previewRelink(
  id: number,
  path: string,
): Promise<RelinkPreview> {
  return invoke<RelinkPreview>("preview_relink", { id, path });
}

export async function listRoots(): Promise<SourceRoot[]> {
  return invoke<SourceRoot[]>("list_roots");
}

export async function listRootStats(): Promise<RootStats[]> {
  return invoke<RootStats[]>("list_root_stats");
}

export async function removeRoot(id: number): Promise<void> {
  return invoke("remove_root", { id });
}

export async function startScan(rootId: number): Promise<void> {
  return invoke("start_scan", { rootId });
}

export async function resumePendingScans(): Promise<void> {
  return invoke("resume_pending_scans");
}

export async function cancelScan(rootId?: number): Promise<void> {
  return invoke("cancel_scan", { rootId: rootId ?? null });
}

export async function pauseScan(): Promise<void> {
  return invoke("pause_scan");
}

export async function resumeScan(): Promise<void> {
  return invoke("resume_scan");
}

export async function rebuildCatalog(): Promise<void> {
  return invoke("rebuild_catalog");
}

export async function getScanStatus(rootId?: number): Promise<ScanProgress> {
  return invoke<ScanProgress>("get_scan_status", {
    rootId: rootId ?? null,
  });
}

export async function listScanStatuses(): Promise<ScanProgress[]> {
  return invoke<ScanProgress[]>("list_scan_statuses");
}

export async function queryAssets(
  filter: AssetFilter,
  sort = "date:desc",
  offset = 0,
  limit = 200,
): Promise<QueryResult> {
  return invoke<QueryResult>("query_assets", { filter, sort, offset, limit });
}

export async function countAssets(filter: AssetFilter): Promise<number> {
  return invoke<number>("count_assets", { filter });
}

export async function getAsset(id: number): Promise<AssetDetail> {
  return invoke<AssetDetail>("get_asset", { id });
}

export async function updateAssetMeta(
  id: number,
  patch: { rating?: number; rotation?: number },
): Promise<AssetDetail> {
  return invoke<AssetDetail>("update_asset_meta", { id, patch });
}

export async function batchAppendTags(
  assetIds: number[],
  tagId: number,
): Promise<number> {
  return invoke<number>("batch_append_tags", { assetIds, tagId });
}

export async function batchRemoveTags(
  assetIds: number[],
  tagId: number,
): Promise<number> {
  return invoke<number>("batch_remove_tags", { assetIds, tagId });
}

export async function batchUpdateAssetMeta(
  ids: number[],
  patch: { rating?: number; rotation?: number },
): Promise<number> {
  return invoke<number>("batch_update_asset_meta", { ids, patch });
}

export async function softDeleteAssets(ids: number[]): Promise<number> {
  return invoke<number>("soft_delete_assets", { ids });
}

export async function restoreAssets(ids: number[]): Promise<number> {
  return invoke<number>("restore_assets", { ids });
}

export async function purgeDelete(
  ids: number[],
  confirmToken: string,
): Promise<number> {
  return invoke<number>("purge_delete", { ids, confirmToken });
}

export async function listTags(): Promise<TagDto[]> {
  return invoke<TagDto[]>("list_tags");
}

export async function createTag(
  name: string,
  parentId?: number,
  color?: string,
): Promise<TagDto> {
  return invoke<TagDto>("create_tag", { name, parentId, color });
}

export async function updateTag(
  id: number,
  name: string,
  color?: string,
): Promise<TagDto> {
  return invoke<TagDto>("update_tag", { id, name, color });
}

export async function deleteTag(id: number): Promise<void> {
  return invoke("delete_tag", { id });
}

export async function listSmartCollections(): Promise<SmartCollection[]> {
  return invoke<SmartCollection[]>("list_smart_collections");
}

export async function saveSmartCollection(
  name: string,
  filter: AssetFilter,
): Promise<SmartCollection> {
  return invoke<SmartCollection>("save_smart_collection", { name, filter });
}

export async function deleteSmartCollection(id: number): Promise<void> {
  return invoke("delete_smart_collection", { id });
}

export async function listAlbums(): Promise<Album[]> {
  return invoke<Album[]>("list_albums");
}

export async function createAlbum(
  name: string,
  sortMode?: string,
  emoji?: string,
): Promise<Album> {
  return invoke<Album>("create_album", { name, sortMode, emoji });
}

export async function updateAlbum(
  id: number,
  name: string,
  emoji?: string,
): Promise<Album> {
  return invoke<Album>("update_album", { id, name, emoji });
}

export async function deleteAlbum(id: number): Promise<void> {
  return invoke("delete_album", { id });
}

export async function setAlbumItems(
  albumId: number,
  assetIds: number[],
): Promise<void> {
  return invoke("set_album_items", { albumId, assetIds });
}

export async function addAlbumItems(
  albumId: number,
  assetIds: number[],
): Promise<number> {
  return invoke<number>("add_album_items", { albumId, assetIds });
}

export async function removeAlbumItems(
  albumId: number,
  assetIds: number[],
): Promise<number> {
  return invoke<number>("remove_album_items", { albumId, assetIds });
}

export async function startExport(
  assetIds: number[],
  destination: string,
  options?: ExportOptions,
  jobId?: number,
): Promise<number> {
  const id = jobId ?? Date.now();
  await invoke("start_export", {
    assetIds,
    destination,
    options,
    jobId: id,
  });
  return id;
}

export async function cancelExport(): Promise<void> {
  return invoke("cancel_export");
}

export async function getExportStatus(): Promise<ExportStatus> {
  return invoke<ExportStatus>("get_export_status");
}

export async function listExportJobs(): Promise<ExportJobSummary[]> {
  return invoke<ExportJobSummary[]>("list_export_jobs");
}

export async function getAlbumAssetIds(albumId: number): Promise<number[]> {
  return invoke<number[]>("get_album_asset_ids", { albumId });
}

export function onScanProgress(
  handler: (event: ScanProgress) => void,
): Promise<UnlistenFn> {
  return listen<ScanProgress>("scan://progress", (e) => handler(e.payload));
}

export function onScanThumbs(
  handler: (event: ScanThumbsEvent) => void,
): Promise<UnlistenFn> {
  return listen<ScanThumbsEvent>("scan://thumbs", (e) => handler(e.payload));
}

export function onJobProgress(
  handler: (event: JobProgress) => void,
): Promise<UnlistenFn> {
  return listen<JobProgress>("job://progress", (e) => handler(e.payload));
}

import type { MessageEnvelope } from "../lib/message/types";

export type { MessageEnvelope } from "../lib/message/types";

export function onMessageNotify(
  handler: (event: MessageEnvelope) => void,
): Promise<UnlistenFn> {
  return listen<MessageEnvelope>("message://notify", (e) => handler(e.payload));
}

export async function queryAssetActivity(
  assetId: number,
  offset = 0,
  limit = 50,
): Promise<ActivityEntry[]> {
  return invoke<ActivityEntry[]>("query_asset_activity", {
    assetId,
    offset,
    limit,
  });
}

export async function undoActivity(activityId: number): Promise<number> {
  return invoke<number>("undo_activity", { activityId });
}
