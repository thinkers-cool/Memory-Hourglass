import { fireEvent, render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { sampleVideoCard } from "../../test/fixtures";
import { VideoSlide } from "./VideoSlide";

describe("VideoSlide", () => {
  it("renders video and calls onEnded", () => {
    const onEnded = vi.fn();
    render(
      <VideoSlide
        card={sampleVideoCard}
        playing={false}
        muted
        onEnded={onEnded}
      />,
    );
    const el = document.querySelector("video");
    expect(el).toHaveAttribute("src", "asset:///tmp/2.mp4");
    fireEvent.ended(el!);
    expect(onEnded).toHaveBeenCalledTimes(1);
  });

  it("plays and pauses with the playing flag", async () => {
    const play = vi.fn().mockRejectedValue(new Error("blocked"));
    const pause = vi.fn();
    const { rerender } = render(
      <VideoSlide
        card={sampleVideoCard}
        playing={true}
        muted={false}
        onEnded={vi.fn()}
      />,
    );
    const el = document.querySelector("video") as HTMLVideoElement;
    el.play = play;
    el.pause = pause;
    rerender(
      <VideoSlide
        card={sampleVideoCard}
        playing={false}
        muted={true}
        onEnded={vi.fn()}
      />,
    );
    expect(pause).toHaveBeenCalled();
    rerender(
      <VideoSlide
        card={{ ...sampleVideoCard, id: 9 }}
        playing={true}
        muted={true}
        onEnded={vi.fn()}
      />,
    );
    expect(play).toHaveBeenCalled();
  });
});
