import { useCallback, useEffect, useRef, useState } from "react";
import {
  buildShuffleOrder,
  resolveNextIndex,
  resolveShuffleIndex,
} from "../lib/slideshow/navigation";
import { useSlideshowPreload } from "../lib/slideshow/preload";
import {
  loadSlideshowSettings,
  saveSlideshowSettings,
} from "../lib/slideshow/settings";
import {
  nextInterval,
  nextTheme,
  THEME_CONFIG,
  VIDEO_END_PADDING_MS,
} from "../lib/slideshow/timing";
import type { SlideshowSettings, SlideshowTheme } from "../lib/slideshow/types";
import type { AssetCard } from "../types";
import { useRafTransition } from "./useReducedMotion";

export function useSlideshow({
  items,
  index,
  onNavigate,
  reducedMotion,
}: {
  items: AssetCard[];
  index: number;
  onNavigate: (index: number) => void;
  reducedMotion: boolean;
}) {
  const [settings, setSettings] = useState(loadSlideshowSettings);
  const [playing, setPlaying] = useState(true);
  const [shuffleOrder, setShuffleOrder] = useState<number[]>([]);
  const [transition, setTransition] = useState<{
    from: number;
    to: number;
  } | null>(null);
  const lastIndexRef = useRef(index);
  const dwellTimerRef = useRef<number | null>(null);
  const videoTimerRef = useRef<number | null>(null);

  const theme: SlideshowTheme = reducedMotion ? "dissolve" : settings.theme;
  const transitionMs = reducedMotion ? 0 : THEME_CONFIG[theme].transitionMs;
  const kenBurns = !reducedMotion && THEME_CONFIG[theme].kenBurns;
  const isAnimating = transition !== null && transition.from !== transition.to;

  const progress = useRafTransition(isAnimating, transitionMs, () => {
    setTransition(null);
  });

  useSlideshowPreload(items, index);

  useEffect(() => {
    saveSlideshowSettings(settings);
  }, [settings]);

  useEffect(() => {
    if (index === lastIndexRef.current) return;
    setTransition({ from: lastIndexRef.current, to: index });
    lastIndexRef.current = index;
  }, [index]);

  const prevLengthRef = useRef(items.length);
  useEffect(() => {
    if (!settings.shuffle) return;
    if (shuffleOrder.length === 0 || prevLengthRef.current !== items.length) {
      prevLengthRef.current = items.length;
      setShuffleOrder(buildShuffleOrder(items, index));
    }
  }, [settings.shuffle, items, items.length, index, shuffleOrder.length]);

  const toggleShuffle = useCallback(() => {
    setSettings((prev) => {
      const shuffle = !prev.shuffle;
      if (shuffle) {
        setShuffleOrder(buildShuffleOrder(items, index));
      } else {
        setShuffleOrder([]);
      }
      return { ...prev, shuffle };
    });
  }, [items, index]);

  const clearTimers = useCallback(() => {
    if (dwellTimerRef.current !== null) {
      window.clearTimeout(dwellTimerRef.current);
      dwellTimerRef.current = null;
    }
    if (videoTimerRef.current !== null) {
      window.clearTimeout(videoTimerRef.current);
      videoTimerRef.current = null;
    }
  }, []);

  const goBy = useCallback(
    (delta: number) => {
      const next = settings.shuffle
        ? resolveShuffleIndex(shuffleOrder, index, delta, settings.loop)
        : resolveNextIndex(items, index, delta, settings.loop);
      if (next !== null && next !== index) onNavigate(next);
    },
    [settings.shuffle, settings.loop, shuffleOrder, items, index, onNavigate],
  );

  const goNext = useCallback(() => goBy(1), [goBy]);
  const goPrev = useCallback(() => goBy(-1), [goBy]);

  const onVideoEnded = useCallback(() => {
    if (!playing) return;
    clearTimers();
    videoTimerRef.current = window.setTimeout(goNext, VIDEO_END_PADDING_MS);
  }, [playing, goNext, clearTimers]);

  useEffect(() => {
    clearTimers();
    if (!playing || items.length <= 1) return;
    const card = items[index];
    if (!card || card.kind === "video") return;
    dwellTimerRef.current = window.setTimeout(goNext, settings.intervalMs);
    return clearTimers;
  }, [playing, index, settings.intervalMs, items, goNext, clearTimers]);

  useEffect(() => {
    if (!playing) return;
    let lock: WakeLockSentinel | null = null;
    const request = async () => {
      try {
        lock = await navigator.wakeLock?.request("screen");
      } catch {
        lock = null;
      }
    };
    void request();
    return () => {
      void lock?.release();
    };
  }, [playing]);

  const patchSettings = useCallback((patch: Partial<SlideshowSettings>) => {
    setSettings((prev) => ({ ...prev, ...patch }));
  }, []);

  const cycleInterval = useCallback(() => {
    patchSettings({ intervalMs: nextInterval(settings.intervalMs) });
  }, [patchSettings, settings.intervalMs]);

  const cycleTheme = useCallback(() => {
    patchSettings({ theme: nextTheme(settings.theme) });
  }, [patchSettings, settings.theme]);

  const toggleLoop = useCallback(() => {
    patchSettings({ loop: !settings.loop });
  }, [patchSettings, settings.loop]);

  const toggleMute = useCallback(() => {
    patchSettings({ muteVideos: !settings.muteVideos });
  }, [patchSettings, settings.muteVideos]);

  return {
    settings,
    playing,
    setPlaying,
    theme,
    kenBurns,
    transitionMs,
    fromIndex: isAnimating ? transition!.from : null,
    toIndex: index,
    progress: isAnimating ? progress : 1,
    goNext,
    goPrev,
    onVideoEnded,
    cycleInterval,
    cycleTheme,
    toggleLoop,
    toggleShuffle,
    toggleMute,
  };
}
