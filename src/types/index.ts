export interface SourceRoot {
  id: number;
  path: string;
  kind: string;
  scan_policy: string;
  poll_secs: number | null;
  last_scan_at: number | null;
  status: string;
  smb_host: string | null;
  smb_share: string | null;
  smb_username: string | null;
  smb_mounted: number;
}

export type SmbSourceInput =
  | {
      mode: "mounted";
      path: string;
      poll_secs?: number;
    }
  | {
      mode: "connect";
      host: string;
      share: string;
      username: string;
      password: string;
      domain?: string;
      poll_secs?: number;
      sub_path?: string;
    };

export interface SmbConnectRequest {
  host: string;
  share: string;
  username: string;
  password: string;
  domain?: string;
  pollSecs?: number;
  subPath?: string;
}

export interface FolderEntry {
  name: string;
  path: string;
}

export interface SmbListSharesRequest {
  host: string;
  username: string;
  password: string;
}

export interface SmbShareEntry {
  name: string;
  comment: string | null;
}

export interface AssetCard {
  id: number;
  file_name: string;
  ext: string;
  kind: string;
  capture_at: number | null;
  rating: number | null;
  sync_state: string;
  thumb_path: string | null;
  abs_path: string;
  has_duplicate: boolean;
}

export interface AssetMeta {
  asset_id: number;
  capture_at: number | null;
  camera: string | null;
  lens: string | null;
  rating: number | null;
  latitude: number | null;
  longitude: number | null;
  keywords_json: string | null;
}

export interface Asset {
  id: number;
  root_id: number;
  rel_path: string;
  file_name: string;
  ext: string;
  kind: string;
  size: number;
  mtime_ns: number;
  sync_state: string;
}

export interface RawTag {
  name: string;
  value: string;
}

export interface LinkedAsset {
  kind: string;
  id: number;
  file_name: string;
  asset_kind: string;
  root_path: string;
  rel_path: string;
}

export interface DuplicateAsset {
  id: number;
  file_name: string;
  root_path: string;
  rel_path: string;
}

export interface AssetDetail {
  asset: Asset;
  meta: AssetMeta | null;
  abs_path: string;
  display_path: string;
  tag_ids: number[];
  album_ids: number[];
  raw_tags: RawTag[];
  links: LinkedAsset[];
  duplicates: DuplicateAsset[];
}

export interface AssetFilter {
  root_id?: number;
  rating_min?: number;
  sync_states?: string[];
  kind?: string;
  camera?: string;
  capture_from?: number;
  capture_to?: number;
  tag_ids?: number[];
  album_ids?: number[];
  meta_search?: string;
  has_gps?: boolean;
  has_duplicate?: boolean;
  deleted_only?: boolean;
  asset_ids?: number[];
}

export type SortMode = "date" | "name" | "rating" | "path";

export type SortDir = "asc" | "desc";

export interface QueryResult {
  total: number;
  items: AssetCard[];
}

export interface ExportManifest {
  destination: string;
  copied: string[];
  failed: { source: string; error: string }[];
}

export interface ExportOptions {
  flat: boolean;
  rename_template?: string;
  format?: string;
}

export type JobPhase = "started" | "running" | "completed" | "failed";

export interface JobProgress {
  job_id: string;
  done: number;
  total: number;
  phase: JobPhase;
  message: string;
  file_name?: string;
}

export interface ExportStatus {
  job_id: number | null;
  status: string;
  manifest: ExportManifest | null;
}

export interface ExportJobSummary {
  id: number;
  status: string;
  created_at: number;
  copied_count: number;
  failed_count: number;
}

export interface Album {
  id: number;
  name: string;
  sort_mode: string;
  emoji: string | null;
  asset_count: number;
}

export interface SmartCollection {
  id: number;
  name: string;
  filter: AssetFilter;
  asset_count: number;
}

export interface TagDto {
  id: number;
  name: string;
  parent_id: number | null;
  color: string | null;
  asset_count: number;
}

export interface RootStats {
  id: number;
  path: string;
  kind: string;
  status: string;
  scan_policy: string;
  poll_secs: number | null;
  last_scan_at: number | null;
  asset_count: number;
  missing_count: number;
}

export interface RelinkPreview {
  matched: number;
  total_sampled: number;
  new_path: string;
}

export interface ScanProgress {
  root_id: number;
  stage: string;
  scanned: number;
  indexed: number;
}

export interface ScanThumbUpdate {
  asset_id: number;
  thumb_path: string;
}

export interface ScanThumbsEvent {
  root_id: number;
  thumbs: ScanThumbUpdate[];
}

export interface WorkspaceInfo {
  path: string;
  name: string;
  id: string;
  read_only: boolean;
}

export type NotificationKind = "success" | "error" | "info";

export type NotificationAction = {
  label_key: string;
  action: "undo_activity" | "dismiss";
  activity_id?: number;
};

export type Notification = {
  kind: NotificationKind;
  text: string;
  activity_id?: number;
  actions?: NotificationAction[];
};

export interface ActivityEntry {
  id: number;
  seq: number;
  occurred_at: number;
  event_type: string;
  actor: string;
  correlation_id: string | null;
  subject_type: string | null;
  subject_id: number | null;
  subject_key: string | null;
  summary: string | null;
  payload_json: string;
  revert_json: string | null;
  undone_at: number | null;
  reversible: boolean;
}

export interface RecentWorkspace {
  path: string;
  name: string;
  last_opened: number;
  valid: boolean;
  root_count: number;
  album_count: number;
  tag_count: number;
  read_only: boolean;
}
