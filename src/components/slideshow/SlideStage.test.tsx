import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { sampleCard, sampleVideoCard } from "../../test/fixtures";
import { SlideStage } from "./SlideStage";

describe("SlideStage", () => {
  it("renders incoming photo slide", () => {
    render(
      <SlideStage
        items={[sampleCard]}
        fromIndex={null}
        toIndex={0}
        progress={1}
        theme="dissolve"
        kenBurns={false}
        dwellMs={3000}
        playing
        muted
        onVideoEnded={vi.fn()}
      />,
    );
    expect(screen.getByAltText("photo.jpg")).toBeInTheDocument();
  });

  it("returns null when no backdrop card exists", () => {
    const { container } = render(
      <SlideStage
        items={[]}
        fromIndex={null}
        toIndex={0}
        progress={1}
        theme="dissolve"
        kenBurns={false}
        dwellMs={3000}
        playing
        muted
        onVideoEnded={vi.fn()}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders video incoming slide", () => {
    render(
      <SlideStage
        items={[sampleVideoCard]}
        fromIndex={null}
        toIndex={0}
        progress={1}
        theme="push"
        kenBurns={false}
        dwellMs={5000}
        playing
        muted
        onVideoEnded={vi.fn()}
      />,
    );
    expect(document.querySelector("video")).toBeInTheDocument();
  });

  it("renders outgoing layer during transition", () => {
    render(
      <SlideStage
        items={[sampleCard, { ...sampleCard, id: 2, file_name: "two.jpg" }]}
        fromIndex={0}
        toIndex={1}
        progress={0.5}
        theme="fade-zoom"
        kenBurns
        dwellMs={3000}
        playing
        muted
        onVideoEnded={vi.fn()}
      />,
    );
    expect(screen.getByAltText("photo.jpg")).toBeInTheDocument();
    expect(screen.getByAltText("two.jpg")).toBeInTheDocument();
  });

  it("applies ken burns on incoming photo when playing", () => {
    render(
      <SlideStage
        items={[sampleCard]}
        fromIndex={null}
        toIndex={0}
        progress={1}
        theme="ken-burns"
        kenBurns
        dwellMs={8000}
        playing
        muted
        onVideoEnded={vi.fn()}
      />,
    );
    const img = screen.getByAltText("photo.jpg");
    expect(img.className).toContain("animate-ken-burns");
  });

  it("fires onVideoEnded for video slide", () => {
    const onVideoEnded = vi.fn();
    render(
      <SlideStage
        items={[sampleVideoCard]}
        fromIndex={null}
        toIndex={0}
        progress={1}
        theme="dissolve"
        kenBurns={false}
        dwellMs={3000}
        playing
        muted
        onVideoEnded={onVideoEnded}
      />,
    );
    const video = document.querySelector("video");
    video?.dispatchEvent(new Event("ended"));
    expect(onVideoEnded).toHaveBeenCalled();
  });
});
