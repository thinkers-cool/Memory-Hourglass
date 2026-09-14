import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { resetThumbLoadQueueForTests } from "../lib/thumbLoad";
import {
  GRID_CARD_DOUBLE_CLICK_WINDOW_MS,
  resetGridCardClickState,
} from "../lib/gridCardClick";
import { sampleCard, sampleVideoCard } from "../test/fixtures";
import { AssetGridCard } from "./AssetGridCard";

describe("AssetGridCard", () => {
  beforeEach(() => {
    class MockIntersectionObserver {
      private readonly callback: IntersectionObserverCallback;

      constructor(callback: IntersectionObserverCallback) {
        this.callback = callback;
      }

      observe() {
        this.callback(
          [{ isIntersecting: true } as IntersectionObserverEntry],
          this as unknown as IntersectionObserver,
        );
      }

      disconnect() {}
    }
    vi.stubGlobal("IntersectionObserver", MockIntersectionObserver);
  });

  afterEach(() => {
    resetGridCardClickState();
    resetThumbLoadQueueForTests();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("opens full view on double click", () => {
    vi.useFakeTimers();
    const onOpenFullView = vi.fn();
    render(
      <AssetGridCard
        card={sampleCard}
        selected={false}
        onSelect={vi.fn()}
        onOpenFullView={onOpenFullView}
      />,
    );
    const button = screen.getByRole("button");
    fireEvent.click(button);
    fireEvent.click(button);
    expect(onOpenFullView).toHaveBeenCalledTimes(1);
  });

  it("selects on single click after the double-click window", () => {
    vi.useFakeTimers();
    const onSelect = vi.fn();
    render(
      <AssetGridCard
        card={sampleCard}
        selected={false}
        onSelect={onSelect}
        onOpenFullView={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button"));
    expect(onSelect).not.toHaveBeenCalled();
    vi.advanceTimersByTime(GRID_CARD_DOUBLE_CLICK_WINDOW_MS);
    expect(onSelect).toHaveBeenCalledWith(false, false);
  });

  it("shows rating stars and stamp indicator together", () => {
    render(
      <AssetGridCard
        card={{ ...sampleCard, rating: 4 }}
        selected={false}
        stampIndicator="matched"
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(document.querySelector(".lucide-stamp")).toBeInTheDocument();
    expect(screen.getByLabelText("Rating 4")).toHaveTextContent("★★★★");
  });

  it("shows stamp indicator only when matched", () => {
    const { rerender } = render(
      <AssetGridCard
        card={sampleCard}
        selected={false}
        stampIndicator="none"
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(document.querySelector(".lucide-stamp")).not.toBeInTheDocument();

    rerender(
      <AssetGridCard
        card={sampleCard}
        selected={false}
        stampIndicator="matched"
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(document.querySelector(".lucide-stamp")).toBeInTheDocument();
  });

  it("shows selection chrome when selected", async () => {
    render(
      <AssetGridCard
        card={sampleCard}
        selected
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    await waitFor(() =>
      expect(screen.getByAltText("photo.jpg")).toBeInTheDocument(),
    );
  });

  it("renders video poster when thumbnail is missing", () => {
    render(
      <AssetGridCard
        card={sampleVideoCard}
        selected={false}
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(document.querySelector("video")).toBeInTheDocument();
    expect(screen.getByText("MP4")).toBeInTheDocument();
  });

  it("renders placeholder when image has no thumbnail", () => {
    render(
      <AssetGridCard
        card={{ ...sampleCard, thumb_path: null }}
        selected={false}
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(document.querySelector(".lucide-file-image")).toBeInTheDocument();
    expect(screen.getByText("JPG")).toBeInTheDocument();
  });

  it("shows raw extension badge styling", () => {
    render(
      <AssetGridCard
        card={{ ...sampleCard, ext: "cr2", file_name: "photo.cr2" }}
        selected={false}
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(screen.getByText("CR2")).toBeInTheDocument();
  });

  it("shows sync state badges when not ok", () => {
    const { rerender } = render(
      <AssetGridCard
        card={{ ...sampleCard, sync_state: "missing" }}
        selected={false}
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(screen.getByText("Missing")).toBeInTheDocument();

    rerender(
      <AssetGridCard
        card={{ ...sampleCard, sync_state: "new" }}
        selected={false}
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(screen.getByText("New")).toBeInTheDocument();

    rerender(
      <AssetGridCard
        card={{ ...sampleCard, sync_state: "modified" }}
        selected={false}
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(screen.getByText("Modified")).toBeInTheDocument();

    rerender(
      <AssetGridCard
        card={{
          ...sampleCard,
          sync_state: "unknown" as typeof sampleCard.sync_state,
        }}
        selected={false}
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(screen.getByText("unknown")).toBeInTheDocument();
  });

  it("selects with modifier keys on mouse down", () => {
    const onSelect = vi.fn();
    render(
      <AssetGridCard
        card={sampleCard}
        selected={false}
        onSelect={onSelect}
        onOpenFullView={vi.fn()}
      />,
    );
    const button = screen.getByRole("button");
    fireEvent.mouseDown(button, { metaKey: true });
    expect(onSelect).toHaveBeenCalledWith(true, false);
    fireEvent.mouseDown(button, { shiftKey: true });
    expect(onSelect).toHaveBeenCalledWith(false, true);
    fireEvent.mouseDown(button, { ctrlKey: true });
    expect(onSelect).toHaveBeenCalledWith(true, false);
  });

  it("positions rating badge beside stamp indicator", () => {
    const { container, rerender } = render(
      <AssetGridCard
        card={{ ...sampleCard, rating: 3 }}
        selected={false}
        stampIndicator="matched"
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(container.querySelector('[class*="left-8"]')).toBeInTheDocument();

    rerender(
      <AssetGridCard
        card={{ ...sampleCard, rating: 3 }}
        selected={false}
        stampIndicator="none"
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    expect(container.querySelector('[class*="left-1.5"]')).toBeInTheDocument();
  });

  it("uses default extension badge styling for standard images", () => {
    render(
      <AssetGridCard
        card={{ ...sampleCard, ext: "jpg" }}
        selected={false}
        onSelect={vi.fn()}
        onOpenFullView={vi.fn()}
      />,
    );
    const badge = screen.getByText("JPG").closest("span")?.parentElement;
    expect(badge).toHaveClass("border-white/15");
  });

  it("ignores mousedown without modifier keys", () => {
    const onSelect = vi.fn();
    render(
      <AssetGridCard
        card={sampleCard}
        selected={false}
        onSelect={onSelect}
        onOpenFullView={vi.fn()}
      />,
    );
    fireEvent.mouseDown(screen.getByRole("button"));
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("ignores plain click when modifier keys are held", () => {
    const onSelect = vi.fn();
    const onOpenFullView = vi.fn();
    render(
      <AssetGridCard
        card={sampleCard}
        selected={false}
        onSelect={onSelect}
        onOpenFullView={onOpenFullView}
      />,
    );
    fireEvent.click(screen.getByRole("button"), { metaKey: true });
    expect(onSelect).not.toHaveBeenCalled();
    expect(onOpenFullView).not.toHaveBeenCalled();
  });
});
