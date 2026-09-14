import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import * as useZoomableMediaModule from "../hooks/useZoomableMedia";
import { sampleCard, sampleVideoCard } from "../test/fixtures";
import { AssetFullView } from "./AssetFullView";

describe("AssetFullView", () => {
  it("shows position and closes", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <AssetFullView
        card={sampleCard}
        items={[sampleCard]}
        index={0}
        total={3}
        onClose={onClose}
        onNavigateRelative={vi.fn()}
      />,
    );
    expect(screen.getByText(/1 \/ 3 — photo\.jpg/)).toBeInTheDocument();
    await user.click(screen.getByLabelText("Close"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("navigates when clicking the side zones", async () => {
    const user = userEvent.setup();
    const onNavigateRelative = vi.fn();
    render(
      <AssetFullView
        card={sampleCard}
        items={[sampleCard, sampleCard, sampleCard]}
        index={1}
        total={3}
        onClose={vi.fn()}
        onNavigateRelative={onNavigateRelative}
      />,
    );
    await user.click(screen.getByLabelText("Previous"));
    expect(onNavigateRelative).toHaveBeenCalledWith(-1);
    await user.click(screen.getByLabelText("Next"));
    expect(onNavigateRelative).toHaveBeenCalledWith(1);
  });

  it("provides zoom controls", () => {
    render(
      <AssetFullView
        card={sampleCard}
        items={[sampleCard]}
        index={0}
        total={3}
        onClose={vi.fn()}
        onNavigateRelative={vi.fn()}
      />,
    );
    expect(screen.getByLabelText("Zoom in")).toBeInTheDocument();
    expect(screen.getByLabelText("Zoom out")).toBeInTheDocument();
    expect(screen.getByLabelText("Fit to view")).toHaveTextContent("100%");
  });

  it("renders video media", () => {
    render(
      <AssetFullView
        card={sampleVideoCard}
        items={[sampleVideoCard]}
        index={0}
        total={1}
        onClose={vi.fn()}
        onNavigateRelative={vi.fn()}
      />,
    );
    expect(document.querySelector("video")).toBeInTheDocument();
  });

  it("disables navigation at edges", () => {
    render(
      <AssetFullView
        card={sampleCard}
        items={[sampleCard]}
        index={0}
        total={1}
        onClose={vi.fn()}
        onNavigateRelative={vi.fn()}
      />,
    );
    expect(screen.getByLabelText("Previous")).toBeDisabled();
    expect(screen.getByLabelText("Next")).toBeDisabled();
  });

  it("rotates with keyboard shortcuts when handlers are provided", () => {
    const onRotateClockwise = vi.fn();
    const onRotateCounterClockwise = vi.fn();
    render(
      <AssetFullView
        card={sampleCard}
        items={[sampleCard]}
        index={0}
        total={1}
        onClose={vi.fn()}
        onNavigateRelative={vi.fn()}
        onRotateClockwise={onRotateClockwise}
        onRotateCounterClockwise={onRotateCounterClockwise}
      />,
    );
    fireEvent.keyDown(window, { key: "r" });
    fireEvent.keyDown(window, { key: "R", shiftKey: true });
    expect(onRotateClockwise).toHaveBeenCalledTimes(1);
    expect(onRotateCounterClockwise).toHaveBeenCalledTimes(1);
  });

  it("resets zoom when pressing 0", () => {
    const resetTransform = vi.fn();
    vi.spyOn(useZoomableMediaModule, "useZoomableMedia").mockReturnValue({
      ref: { current: null },
      scale: 2,
      onTransform: vi.fn(),
      zoomIn: vi.fn(),
      zoomOut: vi.fn(),
      resetTransform,
    });
    render(
      <AssetFullView
        card={sampleCard}
        items={[sampleCard]}
        index={0}
        total={1}
        onClose={vi.fn()}
        onNavigateRelative={vi.fn()}
      />,
    );
    fireEvent.keyDown(window, { key: "0" });
    expect(resetTransform).toHaveBeenCalledTimes(1);
    vi.restoreAllMocks();
  });

  it("handles zoom keyboard shortcuts", async () => {
    const user = userEvent.setup();
    const resetTransform = vi.fn();
    vi.spyOn(useZoomableMediaModule, "useZoomableMedia").mockReturnValue({
      ref: { current: null },
      scale: 1,
      onTransform: vi.fn(),
      zoomIn: vi.fn(),
      zoomOut: vi.fn(),
      resetTransform,
    });
    render(
      <AssetFullView
        card={sampleCard}
        items={[sampleCard]}
        index={0}
        total={1}
        onClose={vi.fn()}
        onNavigateRelative={vi.fn()}
      />,
    );
    await user.keyboard("=");
    await user.keyboard("+");
    await user.keyboard("-");
    await user.keyboard("0");
    expect(resetTransform).toHaveBeenCalledTimes(1);
    vi.restoreAllMocks();
  });
});
