import { useCallback, useEffect, useMemo, useState } from "react";
import { File, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { convertFileSrc } from "@tauri-apps/api/core";
import * as api from "../../api/client";
import {
  MetadataFieldSearch,
  useFilteredRawTags,
} from "../shared/MetadataSearch";
import { activityEventLabel } from "../../lib/message/render";
import type { ActivityEntry, AssetDetail, TagDto } from "../../types";
import { fileKeywordsNotInCatalog } from "../../lib/fileKeywords";

export function InspectorPanel({
  detail,
  tags,
  onClose,
  onPurge,
  purgeEnabled = true,
  onSelectLink,
  onSoftDeleteDuplicate,
  onUndoActivity,
}: {
  detail: AssetDetail;
  tags: TagDto[];
  onClose: () => void;
  onPurge: () => void;
  purgeEnabled?: boolean;
  onSelectLink: (id: number) => void;
  onSoftDeleteDuplicate: (id: number) => void;
  onUndoActivity?: (activityId: number) => Promise<void>;
}) {
  const { t } = useTranslation(["library", "common"]);
  const [metadataQuery, setMetadataQuery] = useState("");
  const [activity, setActivity] = useState<ActivityEntry[]>([]);
  const filteredTags = useFilteredRawTags(detail.raw_tags, metadataQuery);
  const tagLabels = useMemo(() => {
    const byId = new Map(tags.map((tag) => [tag.id, tag.name]));
    return detail.tag_ids
      .map((tagId) => byId.get(tagId))
      .filter((name): name is string => name !== undefined);
  }, [detail.tag_ids, tags]);
  const fileKeywords = useMemo(
    () =>
      fileKeywordsNotInCatalog(
        detail.meta?.keywords_json,
        tags.map((tag) => tag.name),
      ),
    [detail.meta?.keywords_json, tags],
  );

  const loadActivity = useCallback(async () => {
    try {
      const entries = await api.queryAssetActivity(detail.asset.id);
      setActivity(entries ?? []);
    } catch {
      setActivity([]);
    }
  }, [detail.asset.id]);

  useEffect(() => {
    setMetadataQuery("");
    void loadActivity();
  }, [loadActivity]);

  const handleUndoActivity = async (activityId: number) => {
    await onUndoActivity!(activityId);
    await loadActivity();
  };

  const displayPath = detail.display_path;
  const isVideo = detail.asset.kind === "video";

  return (
    <aside className="surface-panel flex h-full w-full min-h-0 flex-col border-l">
      <div className="navbar min-h-0 px-3 py-2 border-b border-divider-subtle shrink-0">
        <div className="navbar-start min-w-0 flex-1">
          <p className="text-sm font-medium truncate">
            {detail.asset.file_name}
          </p>
        </div>
        <div className="navbar-end">
          <button
            type="button"
            className="btn btn-ghost btn-xs btn-square"
            onClick={onClose}
          >
            ×
          </button>
        </div>
      </div>

      <div className="p-3 border-b border-divider-subtle bg-surface-inset shrink-0">
        {isVideo ? (
          <video
            src={convertFileSrc(displayPath)}
            controls
            className="w-full rounded-lg"
          />
        ) : (
          <img
            src={convertFileSrc(displayPath)}
            alt={detail.asset.file_name}
            className="w-full rounded-lg object-contain max-h-52 bg-surface-inset"
          />
        )}
      </div>

      <div className="flex-1 flex flex-col min-h-0 overflow-hidden p-3 gap-3">
        {tagLabels.length > 0 && (
          <div className="flex flex-wrap gap-1 shrink-0">
            {tagLabels.map((tag) => (
              <span key={tag} className="badge badge-outline badge-sm">
                {tag}
              </span>
            ))}
          </div>
        )}

        {fileKeywords.length > 0 && (
          <div className="shrink-0">
            <div className="mb-1 text-[11px] font-medium opacity-60">
              {t("inspector.fileKeywords")}
            </div>
            <div className="flex flex-wrap gap-1">
              {fileKeywords.map((keyword) => (
                <span
                  key={keyword}
                  className="badge badge-ghost badge-sm opacity-80"
                >
                  {keyword}
                </span>
              ))}
            </div>
          </div>
        )}

        {activity.length > 0 && (
          <div className="collapse collapse-arrow bg-surface-inset-strong rounded-lg shrink-0">
            <input type="checkbox" />
            <div className="collapse-title text-xs font-medium py-2 min-h-0">
              {t("inspector.activity")}
            </div>
            <div className="collapse-content text-[11px] space-y-1.5 max-h-40 overflow-auto">
              {activity.map((entry) => (
                <div
                  key={entry.id}
                  className="flex items-start justify-between gap-2"
                >
                  <div className="min-w-0">
                    <div className="truncate">
                      {activityEventLabel(entry.event_type)}
                    </div>
                    <div className="opacity-45">
                      {new Date(entry.occurred_at * 1000).toLocaleString()}
                    </div>
                  </div>
                  {entry.reversible && onUndoActivity && (
                    <button
                      type="button"
                      className="btn btn-ghost btn-xs shrink-0"
                      onClick={() => void handleUndoActivity(entry.id)}
                    >
                      {t("common:action.undo")}
                    </button>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {detail.links.length > 0 && (
          <div className="collapse collapse-arrow bg-surface-inset-strong rounded-lg shrink-0">
            <input type="checkbox" defaultChecked />
            <div className="collapse-title text-xs font-medium py-2 min-h-0">
              {t("inspector.linkedFiles")}
            </div>
            <div className="collapse-content text-xs space-y-1">
              {detail.links.map((link) => (
                <button
                  key={link.id}
                  type="button"
                  className="link link-hover block truncate"
                  onClick={() => onSelectLink(link.id)}
                >
                  {link.file_name}
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="flex-1 flex flex-col min-h-0 rounded-lg bg-surface-inset-strong">
          <div className="shrink-0 border-b border-divider-subtle px-3 py-1.5">
            <div className="mb-1 text-xs font-medium">
              {t("inspector.metadata")}
            </div>
            <MetadataFieldSearch
              value={metadataQuery}
              onChange={setMetadataQuery}
            />
          </div>
          <div className="flex-1 overflow-auto px-3 pb-3 text-[11px] space-y-1 min-h-0">
            {detail.raw_tags.length === 0 ? (
              <p className="opacity-45">{t("inspector.noMetadata")}</p>
            ) : filteredTags.length === 0 ? (
              <p className="opacity-45">{t("inspector.noMatchingMetadata")}</p>
            ) : (
              filteredTags.map((tag) => (
                <div key={`${tag.name}-${tag.value}`} className="flex gap-2">
                  <span className="opacity-45 shrink-0 w-24 truncate">
                    {tag.name}
                  </span>
                  <span className="truncate">{tag.value}</span>
                </div>
              ))
            )}
          </div>
        </div>

        {detail.duplicates.length > 0 && (
          <div className="shrink-0 rounded-lg bg-surface-inset-strong">
            <div className="border-b border-divider-subtle px-3 py-1.5 text-xs font-medium">
              {t("inspector.duplicates")}
            </div>
            <div className="max-h-40 overflow-auto px-3 py-2 text-[11px] space-y-1">
              {detail.duplicates.map((duplicate) => {
                const fullPath = `${duplicate.root_path}/${duplicate.rel_path}`;
                return (
                  <div key={duplicate.id} className="flex items-center gap-2">
                    <button
                      type="button"
                      className="link link-hover flex min-w-0 flex-1 items-center gap-1.5 truncate text-left"
                      onClick={() => onSelectLink(duplicate.id)}
                    >
                      <File className="h-3 w-3 shrink-0 opacity-50" />
                      {fullPath}
                    </button>
                    <button
                      type="button"
                      className="btn btn-ghost btn-xs btn-square h-6 min-h-0 w-6 shrink-0 text-danger-text hover:bg-danger-subtle hover:text-danger"
                      onClick={() => onSoftDeleteDuplicate(duplicate.id)}
                      aria-label={t("common:aria.deleteDuplicate", {
                        fileName: duplicate.file_name,
                      })}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {purgeEnabled && (
        <div className="flex shrink-0 items-center justify-end gap-1 border-t border-divider-subtle px-3 py-2">
          <button
            type="button"
            className="btn btn-ghost btn-xs h-7 min-h-0 gap-1 px-2 text-xs font-normal text-danger-text hover:bg-danger-subtle hover:text-danger"
            onClick={onPurge}
          >
            <Trash2 className="h-3.5 w-3.5" />
            {t("inspector.purge")}
          </button>
        </div>
      )}
    </aside>
  );
}

export function InspectorPlaceholder({ onClose }: { onClose: () => void }) {
  return (
    <aside className="surface-panel flex h-full w-full min-h-0 flex-col border-l">
      <div className="navbar min-h-0 px-3 py-2 border-b border-divider-subtle shrink-0">
        <div className="navbar-end w-full">
          <button
            type="button"
            className="btn btn-ghost btn-xs btn-square"
            onClick={onClose}
          >
            ×
          </button>
        </div>
      </div>
      <div className="flex flex-1 items-center justify-center">
        <span className="loading loading-spinner loading-md opacity-40" />
      </div>
    </aside>
  );
}
