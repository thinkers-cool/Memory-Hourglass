import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { sampleCard, sampleVideoCard } from "../../test/fixtures";
import { SlideStage } from "./SlideStage";

function mockImageDecode() {
  class MockImage {
    onload: (() => void) | null = null;
    set src(_value: string) {
      queueMicrotask(() => this.onload?.());
    }
    decode = vi.fn().mockResolvedValue(undefined);
  }
  vi.stubGlobal("Image", MockImage as unknown as typeof Image);
}

describe("SlideStage", () => {
  it("renders incoming photo slide", async () => {
    mockImageDecode();
    render(
      <SlideStage
        items={[sampleCard]}
        fromIndex={null}
        toIndex={0}
        progress={1}
        theme="dissolve"
        dwellMs={3000}
        playing
        incomingElapsedMs={0}
        outgoingElapsedMs={0}
        muted
        onVideoEnded={vi.fn()}
      />,
    );
    await waitFor(() =>
      expect(screen.getByAltText("photo.jpg")).toBeInTheDocument(),
    );
    vi.unstubAllGlobals();
  });

  it("returns null when no backdrop card exists", () => {
    const { container } = render(
      <SlideStage
        items={[]}
        fromIndex={null}
        toIndex={0}
        progress={1}
        theme="dissolve"
        dwellMs={3000}
        playing
        incomingElapsedMs={0}
        outgoingElapsedMs={0}
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
        theme="dissolve"
        dwellMs={5000}
        playing
        incomingElapsedMs={0}
        outgoingElapsedMs={0}
        muted
        onVideoEnded={vi.fn()}
      />,
    );
    expect(document.querySelector("video")).toBeInTheDocument();
  });

  it("renders outgoing layer during transition", async () => {
    mockImageDecode();
    render(
      <SlideStage
        items={[sampleCard, { ...sampleCard, id: 2, file_name: "two.jpg" }]}
        fromIndex={0}
        toIndex={1}
        progress={0.5}
        theme="ken-burns"
        dwellMs={3000}
        playing
        incomingElapsedMs={100}
        outgoingElapsedMs={2000}
        muted
        onVideoEnded={vi.fn()}
      />,
    );
    await waitFor(() => {
      expect(screen.getByAltText("photo.jpg")).toBeInTheDocument();
      expect(screen.getByAltText("two.jpg")).toBeInTheDocument();
    });
    vi.unstubAllGlobals();
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
        dwellMs={3000}
        playing
        incomingElapsedMs={0}
        outgoingElapsedMs={0}
        muted
        onVideoEnded={onVideoEnded}
      />,
    );
    const video = document.querySelector("video");
    video?.dispatchEvent(new Event("ended"));
    expect(onVideoEnded).toHaveBeenCalled();
  });
});
