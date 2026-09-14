import { afterEach, describe, expect, it, vi } from "vitest";
import {
  GRID_CARD_DOUBLE_CLICK_WINDOW_MS,
  handleGridCardClick,
  resetGridCardClickState,
} from "./gridCardClick";

describe("handleGridCardClick", () => {
  afterEach(() => {
    resetGridCardClickState();
  });

  it("runs single click after the double-click window", () => {
    vi.useFakeTimers();
    const onSingleClick = vi.fn();
    const onDoubleClick = vi.fn();

    handleGridCardClick(1, onSingleClick, onDoubleClick);
    expect(onSingleClick).not.toHaveBeenCalled();
    vi.advanceTimersByTime(GRID_CARD_DOUBLE_CLICK_WINDOW_MS);
    expect(onSingleClick).toHaveBeenCalledTimes(1);
    expect(onDoubleClick).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it("opens full view when the same card is clicked twice quickly", () => {
    const onSingleClick = vi.fn();
    const onDoubleClick = vi.fn();

    handleGridCardClick(1, onSingleClick, onDoubleClick);
    handleGridCardClick(1, onSingleClick, onDoubleClick);

    expect(onSingleClick).not.toHaveBeenCalled();
    expect(onDoubleClick).toHaveBeenCalledTimes(1);
  });

  it("selects both cards when two different cards are clicked quickly", () => {
    vi.useFakeTimers();
    const onSingleClick = vi.fn();
    const onDoubleClick = vi.fn();

    handleGridCardClick(1, onSingleClick, onDoubleClick);
    handleGridCardClick(2, onSingleClick, onDoubleClick);

    expect(onDoubleClick).not.toHaveBeenCalled();
    expect(onSingleClick).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(GRID_CARD_DOUBLE_CLICK_WINDOW_MS);
    expect(onSingleClick).toHaveBeenCalledTimes(2);
    vi.useRealTimers();
  });

  it("treats slow second click on the same card as another single click", () => {
    vi.useFakeTimers();
    const onSingleClick = vi.fn();
    const onDoubleClick = vi.fn();

    handleGridCardClick(1, onSingleClick, onDoubleClick);
    vi.advanceTimersByTime(GRID_CARD_DOUBLE_CLICK_WINDOW_MS);
    handleGridCardClick(1, onSingleClick, onDoubleClick);
    vi.advanceTimersByTime(GRID_CARD_DOUBLE_CLICK_WINDOW_MS);

    expect(onDoubleClick).not.toHaveBeenCalled();
    expect(onSingleClick).toHaveBeenCalledTimes(2);
    vi.useRealTimers();
  });
});
