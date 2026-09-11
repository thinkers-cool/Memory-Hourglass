import { useCallback, useEffect, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  Blend,
  ChevronLeft,
  ChevronRight,
  Clock2,
  Clock3,
  Clock5,
  Clock8,
  PanelRight,
  Pause,
  Play,
  Repeat,
  Scan,
  Shuffle,
  Volume2,
  VolumeX,
  X,
  ZoomIn,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import * as api from "../api/client";
import { SlideStage } from "./slideshow/SlideStage";
import { useReducedMotion } from "../hooks/useReducedMotion";
import { useSlideshowFullscreen } from "../hooks/useSlideshowFullscreen";
import { useSlideshow } from "../hooks/useSlideshow";
import { exitWindowFullscreen } from "../lib/windowFullscreen";
import { coerceIntervalMs, slideshowThemeLabel } from "../lib/slideshow/timing";
import type { SlideshowIntervalMs, SlideshowTheme } from "../lib/slideshow/types";
import type { AssetCard } from "../types";

const ICON_CLASS = "h-4 w-4";

const THEME_ICONS: Record<SlideshowTheme, LucideIcon> = {
  dissolve: Blend,
  "ken-burns": Scan,
  push: PanelRight,
  "fade-zoom": ZoomIn,
};

const INTERVAL_ICONS: Record<SlideshowIntervalMs, LucideIcon> = {
  2000: Clock2,
  3000: Clock3,
  5000: Clock5,
  8000: Clock8,
};

function IconButton({
  label,
  onClick,
  disabled,
  active,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  active?: boolean;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className={`btn btn-circle btn-sm h-8 min-h-0 w-8 border-0 bg-white/10 text-white hover:bg-white/20 ${
        active ? "bg-white/25" : ""
      }`}
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </button>
  );
}

function ThemeIcon({ theme }: { theme: SlideshowTheme }) {
  const Icon = THEME_ICONS[theme];
  return <Icon className={ICON_CLASS} />;
}

function IntervalIcon({ intervalMs }: { intervalMs: number }) {
  const Icon = INTERVAL_ICONS[coerceIntervalMs(intervalMs)];
  return <Icon className={ICON_CLASS} />;
}

export function GalleryPlayer({
  items,
  index,
  onClose,
  onNavigate,
  onRate,
}: {
  items: AssetCard[];
  index: number;
  onClose: () => void;
  onNavigate: (index: number) => void;
  onRate?: (id: number, rating: number) => void;
}) {
  const { t } = useTranslation("library");
  const reducedMotion = useReducedMotion();
  const slideshow = useSlideshow({ items, index, onNavigate, reducedMotion });
  const current = items[index];

  const closeSlideshow = useCallback(() => {
    void exitWindowFullscreen().finally(onClose);
  }, [onClose]);

  useSlideshowFullscreen(closeSlideshow);

  const {
    settings,
    playing,
    setPlaying,
    theme,
    kenBurns,
    fromIndex,
    toIndex,
    progress,
    goNext,
    goPrev,
    onVideoEnded,
    cycleInterval,
    cycleTheme,
    toggleLoop,
    toggleShuffle,
    toggleMute,
  } = slideshow;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        closeSlideshow();
      }
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        goPrev();
      }
      if (e.key === "ArrowRight") {
        e.preventDefault();
        goNext();
      }
      if (e.key === " " || e.code === "Space") {
        e.preventDefault();
        setPlaying((p) => !p);
      }
      if (e.key === "t" || e.key === "T") {
        e.preventDefault();
        cycleTheme();
      }
      if (e.key === "i" || e.key === "I") {
        e.preventDefault();
        cycleInterval();
      }
      if (e.key === "m" || e.key === "M") {
        e.preventDefault();
        toggleMute();
      }
      if (e.key >= "1" && e.key <= "5") {
        e.preventDefault();
        if (current) {
          if (onRate) {
            onRate(current.id, Number(e.key));
          } else {
            void api.updateAssetMeta(current.id, { rating: Number(e.key) });
          }
        }
      }
    };
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  }, [
    closeSlideshow,
    goPrev,
    goNext,
    current,
    setPlaying,
    cycleTheme,
    cycleInterval,
    toggleMute,
    onRate,
  ]);

  if (!current) return null;

  const intervalLabel = `${settings.intervalMs / 1000}s`;
  const effectLabel = slideshowThemeLabel(theme);
  const atStart = index <= 0;
  const atEnd = index >= items.length - 1;

  return (
    <div className="fixed inset-0 z-50 bg-black">
      <div className="absolute inset-0 pointer-events-none">
        <SlideStage
          items={items}
          fromIndex={fromIndex}
          toIndex={toIndex}
          progress={progress}
          theme={theme}
          kenBurns={kenBurns}
          dwellMs={settings.intervalMs}
          playing={playing}
          muted={settings.muteVideos}
          onVideoEnded={onVideoEnded}
        />
      </div>

      <div className="pointer-events-none absolute inset-x-0 top-0 z-10 p-4">
        <p className="pointer-events-auto truncate text-sm text-white/90 drop-shadow-md">
          {t("slideshow.counter", {
            index: index + 1,
            total: items.length,
            fileName: current.file_name,
          })}
          {" · "}
          {effectLabel}
          {" · "}
          {intervalLabel}
          {!playing && ` · ${t("slideshow.paused")}`}
          {reducedMotion && ` · ${t("slideshow.reducedMotion")}`}
        </p>
      </div>

      <div className="pointer-events-none absolute inset-x-0 bottom-0 z-10 flex justify-end p-4">
        <div className="pointer-events-auto flex items-center gap-1 rounded-full bg-black/50 p-1.5 backdrop-blur-sm">
          <IconButton label={t("slideshow.previous")} onClick={goPrev} disabled={!settings.loop && atStart}>
            <ChevronLeft className={ICON_CLASS} />
          </IconButton>
          <IconButton
            label={playing ? t("slideshow.pause") : t("slideshow.play")}
            onClick={() => setPlaying((p) => !p)}
          >
            {playing ? (
              <Pause className={ICON_CLASS} />
            ) : (
              <Play className={ICON_CLASS} />
            )}
          </IconButton>
          <IconButton label={t("slideshow.next")} onClick={goNext} disabled={!settings.loop && atEnd}>
            <ChevronRight className={ICON_CLASS} />
          </IconButton>
          <span className="mx-0.5 h-5 w-px bg-white/20" />
          <IconButton
            label={t("slideshow.interval", { seconds: intervalLabel })}
            onClick={cycleInterval}
            active={settings.intervalMs !== 3000}
          >
            <IntervalIcon intervalMs={settings.intervalMs} />
          </IconButton>
          <IconButton
            label={
              reducedMotion
                ? t("slideshow.effectsDisabled")
                : t("slideshow.effect", { effect: effectLabel })
            }
            onClick={cycleTheme}
            disabled={reducedMotion}
            active={theme !== "dissolve"}
          >
            <ThemeIcon theme={theme} />
          </IconButton>
          <IconButton
            label={settings.muteVideos ? t("slideshow.unmuteVideos") : t("slideshow.muteVideos")}
            onClick={toggleMute}
            active={!settings.muteVideos}
          >
            {settings.muteVideos ? (
              <VolumeX className={ICON_CLASS} />
            ) : (
              <Volume2 className={ICON_CLASS} />
            )}
          </IconButton>
          <IconButton label={t("slideshow.loop")} onClick={toggleLoop} active={settings.loop}>
            <Repeat className={ICON_CLASS} />
          </IconButton>
          <IconButton label={t("slideshow.shuffle")} onClick={toggleShuffle} active={settings.shuffle}>
            <Shuffle className={ICON_CLASS} />
          </IconButton>
          <span className="mx-0.5 h-5 w-px bg-white/20" />
          <IconButton label={t("slideshow.close")} onClick={closeSlideshow}>
            <X className={ICON_CLASS} />
          </IconButton>
        </div>
      </div>
    </div>
  );
}
