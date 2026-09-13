import { convertFileSrc } from "@tauri-apps/api/core";
import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useZoomableMedia } from "../hooks/useZoomableMedia";
import {
  handleImageViewerRotateKey,
  handleImageViewerZoomKey,
} from "../lib/imageViewerZoom";
import { imageRotationStyle, resolveRotation } from "../lib/imageRotation";
import { ImageViewerRotateControls } from "./shared/ImageViewerRotateControls";
import { ImageViewerZoomControls } from "./shared/ImageViewerZoomControls";
import { IconTooltip } from "./shared/Tooltip";
import { ZoomableMedia } from "./shared/ZoomableMedia";
import type { AssetCard } from "../types";

function NavZone({
  label,
  disabled,
  onClick,
}: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <IconTooltip
      tip={label}
      placement="top"
      className="flex h-full w-1/4 min-h-0 shrink-0"
    >
      <button
        type="button"
        className="btn btn-ghost pointer-events-auto h-full w-full min-h-0 rounded-none border-0 bg-transparent p-0 shadow-none transition-colors hover:bg-base-content/5 disabled:pointer-events-none disabled:bg-transparent"
        onClick={onClick}
        disabled={disabled}
        aria-label={label}
      />
    </IconTooltip>
  );
}

export function AssetFullView({
  card,
  index,
  total,
  rotation,
  onClose,
  onNavigateRelative,
  onRotateClockwise,
  onRotateCounterClockwise,
}: {
  card: AssetCard;
  index: number;
  total: number;
  rotation?: number | null;
  onClose: () => void;
  onNavigateRelative: (delta: number) => void;
  onRotateClockwise?: () => void;
  onRotateCounterClockwise?: () => void;
}) {
  const { t } = useTranslation(["library", "common"]);
  const zoom = useZoomableMedia();
  const canRotate =
    card.kind === "image" &&
    onRotateClockwise !== undefined &&
    onRotateCounterClockwise !== undefined;
  const displayRotation = resolveRotation(rotation ?? card.rotation);

  const goPrev = useCallback(
    () => onNavigateRelative(-1),
    [onNavigateRelative],
  );
  const goNext = useCallback(() => onNavigateRelative(1), [onNavigateRelative]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (
        canRotate &&
        handleImageViewerRotateKey(event, {
          rotateClockwise: onRotateClockwise!,
          rotateCounterClockwise: onRotateCounterClockwise!,
        })
      ) {
        return;
      }
      handleImageViewerZoomKey(event, zoom);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    canRotate,
    onRotateClockwise,
    onRotateCounterClockwise,
    zoom,
    zoom.zoomIn,
    zoom.zoomOut,
    zoom.resetTransform,
  ]);

  const mediaSrc = convertFileSrc(card.abs_path);

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col app-canvas">
      <div className="navbar min-h-0 shrink-0 border-b border-divider-subtle px-3 py-1">
        <div className="navbar-start min-w-0">
          <span className="truncate text-sm">
            {t("fullView.counter", {
              index: index + 1,
              total,
              fileName: card.file_name,
            })}
          </span>
        </div>
        <div className="navbar-end gap-1">
          {canRotate ? (
            <ImageViewerRotateControls
              onRotateClockwise={onRotateClockwise!}
              onRotateCounterClockwise={onRotateCounterClockwise!}
            />
          ) : null}
          <ImageViewerZoomControls
            zoom={zoom.scale}
            onZoomIn={zoom.zoomIn}
            onZoomOut={zoom.zoomOut}
            onReset={zoom.resetTransform}
          />
          <button
            type="button"
            className="btn btn-ghost btn-interactive btn-sm btn-square h-8 min-h-0 w-8"
            onClick={onClose}
            aria-label={t("common:aria.close")}
          >
            ×
          </button>
        </div>
      </div>

      <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
        <div className="absolute inset-0 z-0">
          <ZoomableMedia
            className="relative flex h-full min-h-0 w-full flex-col"
            resetKey={card.id}
            transformRef={zoom.ref}
            onTransform={zoom.onTransform}
          >
            {card.kind === "video" ? (
              <video
                src={mediaSrc}
                controls
                className="max-h-[calc(100vh-12rem)] max-w-full object-contain"
              />
            ) : (
              <img
                src={mediaSrc}
                alt={card.file_name}
                className="max-h-[calc(100vh-12rem)] max-w-full object-contain"
                style={imageRotationStyle(displayRotation)}
                draggable={false}
              />
            )}
          </ZoomableMedia>
        </div>

        <div className="pointer-events-none absolute inset-0 z-10 flex">
          <NavZone
            label={t("fullView.previous")}
            onClick={goPrev}
            disabled={index <= 0}
          />
          <div className="flex-1" aria-hidden="true" />
          <NavZone
            label={t("fullView.next")}
            onClick={goNext}
            disabled={index >= total - 1}
          />
        </div>
      </div>
    </div>
  );
}
