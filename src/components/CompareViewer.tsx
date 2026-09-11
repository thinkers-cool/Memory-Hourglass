import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useZoomableMedia } from "../hooks/useZoomableMedia";
import type { Album, AssetCard, TagDto } from "../types";
import { CompareItemBar } from "./CompareItemBar";
import { ImageViewerZoomControls } from "./shared/ImageViewerZoomControls";
import { ZoomableMedia } from "./shared/ZoomableMedia";

function ComparePane({
  item,
  index,
  total,
  showClose,
  onClose,
  tags,
  albums,
  busy,
  compareDetails,
  tagMenuId,
  albumMenuId,
  onTagMenuOpenChange,
  onAlbumMenuOpenChange,
  onRate,
  onToggleTag,
  onCreateTag,
  onToggleAlbum,
  onCreateAlbum,
  onExport,
  onDelete,
}: {
  item: AssetCard;
  index: number;
  total: number;
  showClose: boolean;
  onClose: () => void;
  tags: TagDto[];
  albums: Album[];
  busy: boolean;
  compareDetails: Record<number, { tag_ids: number[]; album_ids: number[] }>;
  tagMenuId: number | null;
  albumMenuId: number | null;
  onTagMenuOpenChange: (id: number, open: boolean) => void;
  onAlbumMenuOpenChange: (id: number, open: boolean) => void;
  onRate: (id: number, rating: number) => void;
  onToggleTag: (
    id: number,
    tagId: number,
    add: boolean,
  ) => void | Promise<void>;
  onCreateTag: (
    id: number,
    tagName: string,
  ) => string | void | Promise<string | void>;
  onToggleAlbum: (
    id: number,
    albumId: number,
    add: boolean,
  ) => void | Promise<void>;
  onCreateAlbum: (
    id: number,
    name: string,
  ) => string | void | Promise<string | void>;
  onExport: (id: number) => void;
  onDelete: (id: number) => void;
}) {
  const { t } = useTranslation(["library", "common"]);
  const zoom = useZoomableMedia();
  const mediaSrc = convertFileSrc(item.abs_path);

  return (
    <div className="flex h-full min-h-0 min-w-0 flex-col border-l border-divider-subtle first:border-l-0">
      <div className="navbar min-h-0 shrink-0 border-b border-divider-subtle px-3 py-1">
        <div className="navbar-start min-w-0">
          <span className="truncate text-sm">
            {t("compare.counter", {
              index: index + 1,
              total,
              fileName: item.file_name,
            })}
          </span>
        </div>
        <div className="navbar-end gap-1">
          <ImageViewerZoomControls
            zoom={zoom.scale}
            onZoomIn={zoom.zoomIn}
            onZoomOut={zoom.zoomOut}
            onReset={zoom.resetTransform}
          />
          {showClose && (
            <button
              type="button"
              className="btn btn-ghost btn-interactive btn-sm btn-square h-8 min-h-0 w-8"
              onClick={onClose}
              aria-label={t("common:aria.close")}
            >
              ×
            </button>
          )}
        </div>
      </div>

      <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
        <ZoomableMedia
          resetKey={item.id}
          transformRef={zoom.ref}
          onTransform={zoom.onTransform}
        >
          {item.kind === "video" ? (
            <video
              src={mediaSrc}
              controls
              className="max-h-full max-w-full object-contain"
            />
          ) : (
            <img
              src={mediaSrc}
              alt={item.file_name}
              className="max-h-full max-w-full object-contain"
              draggable={false}
            />
          )}
        </ZoomableMedia>

        <div className="pointer-events-none absolute inset-x-0 bottom-3 flex justify-center px-2">
          <CompareItemBar
            rating={item.rating}
            tags={tags}
            albums={albums}
            busy={busy}
            tagMenuOpen={tagMenuId === item.id}
            onTagMenuOpenChange={(open) => onTagMenuOpenChange(item.id, open)}
            albumMenuOpen={albumMenuId === item.id}
            onAlbumMenuOpenChange={(open) =>
              onAlbumMenuOpenChange(item.id, open)
            }
            onRate={(rating) => onRate(item.id, rating)}
            onToggleTag={(tagId, add) => onToggleTag(item.id, tagId, add)}
            onCreateTag={(tagName) => onCreateTag(item.id, tagName)}
            onToggleAlbum={(albumId, add) =>
              onToggleAlbum(item.id, albumId, add)
            }
            onCreateAlbum={(name) => onCreateAlbum(item.id, name)}
            onExport={() => onExport(item.id)}
            onDelete={() => onDelete(item.id)}
            selectedTagKeys={(compareDetails[item.id]?.tag_ids ?? []).map(
              String,
            )}
            selectedAlbumKeys={(compareDetails[item.id]?.album_ids ?? []).map(
              String,
            )}
          />
        </div>
      </div>
    </div>
  );
}

export function CompareViewer({
  items,
  tags,
  albums,
  busy,
  compareDetails,
  stampArmed,
  onToggleStamp,
  onClose,
  onRate,
  onToggleTag,
  onCreateTag,
  onToggleAlbum,
  onCreateAlbum,
  onExport,
  onDelete,
}: {
  items: AssetCard[];
  tags: TagDto[];
  albums: Album[];
  busy: boolean;
  compareDetails: Record<number, { tag_ids: number[]; album_ids: number[] }>;
  stampArmed: boolean;
  onToggleStamp: () => void;
  onClose: () => void;
  onRate: (id: number, rating: number) => void;
  onToggleTag: (
    id: number,
    tagId: number,
    add: boolean,
  ) => void | Promise<void>;
  onCreateTag: (
    id: number,
    tagName: string,
  ) => string | void | Promise<string | void>;
  onToggleAlbum: (
    id: number,
    albumId: number,
    add: boolean,
  ) => void | Promise<void>;
  onCreateAlbum: (
    id: number,
    name: string,
  ) => string | void | Promise<string | void>;
  onExport: (id: number) => void;
  onDelete: (id: number) => void;
}) {
  const [tagMenuId, setTagMenuId] = useState<number | null>(null);
  const [albumMenuId, setAlbumMenuId] = useState<number | null>(null);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
        return;
      }
      if ((event.key === " " || event.code === "Space") && stampArmed) {
        event.preventDefault();
        onToggleStamp();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, onToggleStamp, stampArmed]);

  if (items.length < 2) {
    return null;
  }

  const columns = Math.min(items.length, 4);

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col app-canvas">
      <div
        className="grid min-h-0 flex-1 grid-rows-1"
        style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
      >
        {items.map((item, index) => (
          <ComparePane
            key={item.id}
            item={item}
            index={index}
            total={items.length}
            showClose={index === items.length - 1}
            onClose={onClose}
            tags={tags}
            albums={albums}
            busy={busy}
            compareDetails={compareDetails}
            tagMenuId={tagMenuId}
            albumMenuId={albumMenuId}
            onTagMenuOpenChange={(id, open) => {
              setTagMenuId(open ? id : null);
              if (open) setAlbumMenuId(null);
            }}
            onAlbumMenuOpenChange={(id, open) => {
              setAlbumMenuId(open ? id : null);
              if (open) setTagMenuId(null);
            }}
            onRate={onRate}
            onToggleTag={onToggleTag}
            onCreateTag={onCreateTag}
            onToggleAlbum={onToggleAlbum}
            onCreateAlbum={onCreateAlbum}
            onExport={onExport}
            onDelete={onDelete}
          />
        ))}
      </div>
    </div>
  );
}
