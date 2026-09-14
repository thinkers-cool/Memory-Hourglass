import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { sampleCard, sampleVideoCard } from "../test/fixtures";
import * as fullViewMedia from "../lib/fullViewMedia";
import { useFullViewDisplayCard } from "./useFullViewDisplayCard";

describe("useFullViewDisplayCard", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("keeps the previous card visible until the next image is ready", async () => {
    const nextCard = { ...sampleCard, id: 2, file_name: "next.jpg", abs_path: "/tmp/2.jpg" };
    let resolveNext: (() => void) | undefined;
    vi.spyOn(fullViewMedia, "preloadFullViewImage").mockImplementation((path) => {
      if (path === nextCard.abs_path) {
        return new Promise<void>((resolve) => {
          resolveNext = resolve;
        });
      }
      return Promise.resolve();
    });
    vi.spyOn(fullViewMedia, "prefetchFullViewNeighbors").mockImplementation(() => {});

    const { result, rerender } = renderHook(
      ({ card, neighbors }) => useFullViewDisplayCard(card, neighbors),
      {
        initialProps: { card: sampleCard, neighbors: [nextCard] },
      },
    );

    rerender({ card: nextCard, neighbors: [sampleCard] });
    expect(result.current.id).toBe(sampleCard.id);

    resolveNext?.();
    await waitFor(() => {
      expect(result.current.id).toBe(nextCard.id);
    });
  });

  it("switches videos immediately", () => {
    vi.spyOn(fullViewMedia, "prefetchFullViewNeighbors").mockImplementation(() => {});

    const { result, rerender } = renderHook(
      ({ card, neighbors }) => useFullViewDisplayCard(card, neighbors),
      {
        initialProps: { card: sampleCard, neighbors: [] },
      },
    );

    rerender({ card: sampleVideoCard, neighbors: [] });
    expect(result.current).toBe(sampleVideoCard);
  });
});
