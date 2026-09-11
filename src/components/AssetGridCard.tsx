import { convertFileSrc } from "@tauri-apps/api/core";
import { Check, FileImage, Film, Image, Stamp } from "lucide-react";
import type { StampIndicatorState } from "../../lib/stamp";
import type { AssetCard } from "../types";
import { RatingStarsBadge } from "./shared/RatingStarsBadge";
import { VideoPoster } from "./shared/VideoPoster";
import { handleGridCardClick } from "../lib/gridCardClick";

const SYNC_BADGE: Record<string, string> = {
  missing: "badge-error",
  new: "badge-success",
  modified: "badge-warning",
};

const RAW_EXTENSIONS = new Set([
  "arw",
  "cr2",
  "cr3",
  "dng",
  "nef",
  "orf",
  "pef",
  "raf",
  "rw2",
  "srw",
  "raw",
]);

function extBadgeClass(kind: string, ext: string): string {
  if (kind === "video") {
    return "border-violet-400/30 bg-violet-600/85 text-white shadow-violet-950/25";
  }
  if (RAW_EXTENSIONS.has(ext.toLowerCase())) {
    return "border-amber-300/35 bg-amber-500/90 text-amber-950 shadow-amber-950/20";
  }
  return "border-white/15 bg-neutral-950/75 text-white/95 shadow-black/30";
}

function FileExtBadge({ kind, ext }: { kind: string; ext: string }) {
  const label = ext.toUpperCase();
  const Icon = kind === "video" ? Film : Image;
  return (
    <span
      className={`pointer-events-none absolute bottom-1.5 right-1.5 z-[15] inline-flex max-w-[calc(100%-12px)] items-center gap-0.5 rounded-md border px-1.5 py-0.5 text-[9px] font-semibold leading-none tracking-[0.08em] backdrop-blur-md shadow-sm ${extBadgeClass(kind, ext)}`}
    >
      <Icon className="h-2.5 w-2.5 shrink-0 opacity-90" strokeWidth={2.25} aria-hidden />
      <span className="truncate">{label}</span>
    </span>
  );
}

export function AssetGridCard({
  card,
  selected,
  stampIndicator = "none",
  onSelect,
  onOpenFullView,
}: {
  card: AssetCard;
  selected: boolean;
  stampIndicator?: StampIndicatorState;
  onSelect: (multi: boolean, range: boolean) => void;
  onOpenFullView: () => void;
}) {
  return (
    <button
      type="button"
      className={`btn btn-ghost relative block aspect-square h-auto min-h-0 w-full overflow-hidden rounded-lg border-0 bg-surface-inset p-0 text-left shadow-none ring-2 ring-inset transition-shadow group ${
        selected ? "ring-primary" : "ring-transparent hover:ring-[var(--mem-ring-hover)]"
      }`}
      style={{ viewTransitionName: `asset-${card.id}` }}
      onMouseDown={(e) => {
        if (!(e.shiftKey || e.metaKey || e.ctrlKey)) return;
        e.preventDefault();
        onSelect(e.metaKey || e.ctrlKey, e.shiftKey);
      }}
      onClick={(e) => {
        if (e.shiftKey || e.metaKey || e.ctrlKey) return;
        handleGridCardClick(
          card.id,
          () => onSelect(false, false),
          onOpenFullView,
        );
      }}
    >
      {selected && (
        <span
          className="pointer-events-none absolute top-0 right-0 z-30 flex h-9 w-9 items-start justify-end"
          aria-hidden
        >
          <span className="absolute top-0 right-0 h-0 w-0 border-t-[36px] border-l-[36px] border-t-primary border-l-transparent" />
          <Check className="relative mt-1 mr-1 h-4 w-4 stroke-[2.5] text-primary-content" />
        </span>
      )}
      <figure className="relative m-0 h-full w-full">
        {card.thumb_path ? (
          <img
            src={convertFileSrc(card.thumb_path)}
            alt={card.file_name}
            className="w-full h-full object-cover"
            loading="lazy"
          />
        ) : card.kind === "video" ? (
          <VideoPoster
            src={card.abs_path}
            label={card.file_name}
            className="pointer-events-none h-full w-full object-cover"
          />
        ) : (
          <span className="flex h-full w-full items-center justify-center bg-surface-inset-strong">
            <FileImage className="h-8 w-8 text-placeholder" strokeWidth={1.25} aria-hidden />
          </span>
        )}

        <FileExtBadge kind={card.kind} ext={card.ext} />

        {card.sync_state !== "ok" && (
          <span className={`absolute top-1.5 right-1.5 badge badge-xs ${SYNC_BADGE[card.sync_state] ?? "badge-ghost"}`}>
            {card.sync_state}
          </span>
        )}

        {stampIndicator === "matched" && (
          <span
            className="pointer-events-none absolute bottom-1.5 left-1.5 z-[15] inline-flex h-5 w-5 items-center justify-center rounded-md border border-primary/40 bg-primary/90 text-primary-content shadow-sm backdrop-blur-md"
            aria-hidden
          >
            <Stamp className="h-3 w-3" strokeWidth={2.25} />
          </span>
        )}

        {card.rating && card.rating > 0 && (
          <span
            className={`pointer-events-none absolute bottom-1.5 z-[15] ${
              stampIndicator === "matched" ? "left-8" : "left-1.5"
            }`}
          >
            <RatingStarsBadge rating={card.rating} />
          </span>
        )}

        <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent px-2 py-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
          <p className="text-[11px] text-white truncate">{card.file_name}</p>
        </div>
      </figure>
    </button>
  );
}
