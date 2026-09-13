import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { sampleCard } from "../test/fixtures";
import { GalleryPlayer } from "./GalleryPlayer";

const slideshowState = vi.hoisted(() => ({
  settings: {
    intervalMs: 3000,
    theme: "dissolve" as const,
    loop: true,
    shuffle: false,
    muteVideos: true,
  },
  playing: true,
  theme: "dissolve" as const,
  fromIndex: null as number | null,
  toIndex: 0,
  progress: 1,
  incomingElapsedMs: 0,
  outgoingElapsedMs: 0,
  goNext: vi.fn(),
  goPrev: vi.fn(),
  onVideoEnded: vi.fn(),
  setPlaying: vi.fn(),
  cycleInterval: vi.fn(),
  cycleTheme: vi.fn(),
  toggleLoop: vi.fn(),
  toggleShuffle: vi.fn(),
  toggleMute: vi.fn(),
}));

vi.mock("../hooks/useReducedMotion", () => ({
  useReducedMotion: vi.fn(() => false),
}));

vi.mock("../lib/windowFullscreen", () => ({
  exitWindowFullscreen: vi.fn(async () => undefined),
}));

vi.mock("../hooks/useSlideshowFullscreen", () => ({
  useSlideshowFullscreen: vi.fn(),
}));

vi.mock("../hooks/useSlideshow", () => ({
  useSlideshow: () => slideshowState,
}));

vi.mock("../api/client", () => ({
  updateAssetMeta: vi.fn(),
}));

import { useReducedMotion } from "../hooks/useReducedMotion";
import * as api from "../api/client";

describe("GalleryPlayer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    slideshowState.playing = true;
    slideshowState.settings.loop = true;
    slideshowState.settings.muteVideos = true;
    slideshowState.settings.intervalMs = 3000;
    slideshowState.theme = "dissolve";
    vi.mocked(useReducedMotion).mockReturnValue(false);
  });

  it("renders slideshow controls and closes on escape", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <GalleryPlayer
        items={[sampleCard]}
        index={0}
        onClose={onClose}
        onNavigate={vi.fn()}
      />,
    );
    expect(screen.getByLabelText("Close")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("returns null when index is out of range", () => {
    const { container } = render(
      <GalleryPlayer
        items={[sampleCard]}
        index={5}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("handles navigation and playback keyboard shortcuts", async () => {
    const user = userEvent.setup();
    render(
      <GalleryPlayer
        items={[sampleCard]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    await user.keyboard("{ArrowLeft}");
    await user.keyboard("{ArrowRight}");
    await user.keyboard(" ");
    const playToggle = slideshowState.setPlaying.mock.calls.at(-1)?.[0];
    if (typeof playToggle === "function") {
      expect(playToggle(true)).toBe(false);
    }
    await user.keyboard("t");
    await user.keyboard("i");
    await user.keyboard("m");
    expect(slideshowState.goPrev).toHaveBeenCalled();
    expect(slideshowState.goNext).toHaveBeenCalled();
    expect(slideshowState.setPlaying).toHaveBeenCalled();
    expect(slideshowState.cycleTheme).toHaveBeenCalled();
    expect(slideshowState.cycleInterval).toHaveBeenCalled();
    expect(slideshowState.toggleMute).toHaveBeenCalled();
  });

  it("rates via onRate callback", async () => {
    const user = userEvent.setup();
    const onRate = vi.fn();
    render(
      <GalleryPlayer
        items={[sampleCard]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
        onRate={onRate}
      />,
    );
    await user.keyboard("4");
    expect(onRate).toHaveBeenCalledWith(1, 4);
  });

  it("updates asset meta when onRate is omitted", async () => {
    const user = userEvent.setup();
    render(
      <GalleryPlayer
        items={[sampleCard]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    await user.keyboard("5");
    expect(api.updateAssetMeta).toHaveBeenCalledWith(1, { rating: 5 });
  });

  it("ignores rating keys when no current slide exists", async () => {
    const user = userEvent.setup();
    render(
      <GalleryPlayer
        items={[]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    await user.keyboard("3");
    expect(api.updateAssetMeta).not.toHaveBeenCalled();
  });

  it("clicks toolbar controls", async () => {
    const user = userEvent.setup();
    slideshowState.playing = false;
    slideshowState.settings.loop = true;
    slideshowState.settings.intervalMs = 5000;
    slideshowState.theme = "ken-burns";
    render(
      <GalleryPlayer
        items={[sampleCard, { ...sampleCard, id: 2 }]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    await user.click(screen.getByLabelText("Previous"));
    const playButton = screen.getByLabelText("Play");
    await user.click(playButton);
    const clickToggle = slideshowState.setPlaying.mock.calls.at(-1)?.[0];
    if (typeof clickToggle === "function") {
      expect(clickToggle(false)).toBe(true);
    }
    await user.click(screen.getByLabelText("Next"));
    await user.click(screen.getByLabelText("Interval: 5s"));
    await user.click(screen.getByLabelText(/Effect:/i));
    await user.click(screen.getByLabelText("Unmute videos"));
    await user.click(screen.getByLabelText("Loop"));
    await user.click(screen.getByLabelText("Shuffle"));
    await user.click(screen.getByLabelText("Close"));
    expect(slideshowState.goPrev).toHaveBeenCalled();
    expect(slideshowState.goNext).toHaveBeenCalled();
    expect(slideshowState.cycleInterval).toHaveBeenCalled();
    expect(slideshowState.cycleTheme).toHaveBeenCalled();
    expect(slideshowState.toggleMute).toHaveBeenCalled();
    expect(slideshowState.toggleLoop).toHaveBeenCalled();
    expect(slideshowState.toggleShuffle).toHaveBeenCalled();
  });

  it("shows unmuted volume icon when videos are not muted", () => {
    slideshowState.settings.muteVideos = false;
    render(
      <GalleryPlayer
        items={[sampleCard]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    expect(screen.getByLabelText("Mute videos")).toBeInTheDocument();
    expect(document.querySelector(".lucide-volume-2")).toBeInTheDocument();
  });

  it("disables theme cycling when reduced motion is enabled", async () => {
    vi.mocked(useReducedMotion).mockReturnValue(true);
    render(
      <GalleryPlayer
        items={[sampleCard]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    expect(screen.getByText(/reduced motion/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Effects disabled/i)).toBeDisabled();
  });

  it("renders transition theme icons", () => {
    for (const theme of [
      "ken-burns",
      "fade-zoom",
      "push",
      "dip-black",
    ] as const) {
      slideshowState.theme = theme;
      const { unmount } = render(
        <GalleryPlayer
          items={[sampleCard]}
          index={0}
          onClose={vi.fn()}
          onNavigate={vi.fn()}
        />,
      );
      expect(screen.getByLabelText(/Effect:/i)).toBeInTheDocument();
      unmount();
    }
  });

  it("renders alternate interval icons", () => {
    slideshowState.settings.intervalMs = 2000;
    const { unmount } = render(
      <GalleryPlayer
        items={[sampleCard]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    expect(screen.getByLabelText(/Interval:/i)).toBeInTheDocument();
    unmount();

    slideshowState.settings.intervalMs = 8000;
    render(
      <GalleryPlayer
        items={[sampleCard]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    expect(screen.getByLabelText(/Interval:/i)).toBeInTheDocument();
  });

  it("disables navigation at edges when loop is off", () => {
    slideshowState.settings.loop = false;
    render(
      <GalleryPlayer
        items={[sampleCard]}
        index={0}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );
    expect(screen.getByLabelText("Previous")).toBeDisabled();
    expect(screen.getByLabelText("Next")).toBeDisabled();
  });
});
