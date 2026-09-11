import { fireEvent, render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VideoPoster } from "./VideoPoster";

describe("VideoPoster", () => {
  it("renders poster video", () => {
    render(<VideoPoster src="/tmp/poster.mp4" label="Preview" />);
    const video = document.querySelector("video");
    expect(video).toHaveAttribute("src", "asset:///tmp/poster.mp4");
    expect(video).toHaveAttribute("aria-label", "Preview");
  });

  it("seeks to first frame on metadata load", () => {
    render(<VideoPoster src="/tmp/poster.mp4" className="poster" />);
    const video = document.querySelector("video") as HTMLVideoElement;
    Object.defineProperty(video, "currentTime", {
      writable: true,
      value: 0,
    });
    fireEvent.loadedMetadata(video);
    expect(video.currentTime).toBe(0.001);
  });

  it("skips seeking when current time is already set", () => {
    render(<VideoPoster src="/tmp/poster.mp4" />);
    const video = document.querySelector("video") as HTMLVideoElement;
    Object.defineProperty(video, "currentTime", {
      writable: true,
      value: 0.5,
    });
    fireEvent.loadedMetadata(video);
    expect(video.currentTime).toBe(0.5);
  });

  it("does not seek again after first metadata load", () => {
    render(<VideoPoster src="/tmp/poster.mp4" />);
    const video = document.querySelector("video") as HTMLVideoElement;
    Object.defineProperty(video, "currentTime", {
      writable: true,
      value: 0,
    });
    fireEvent.loadedMetadata(video);
    video.currentTime = 0.5;
    fireEvent.loadedMetadata(video);
    expect(video.currentTime).toBe(0.5);
  });
});
