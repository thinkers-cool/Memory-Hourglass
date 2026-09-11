import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  dismissToast,
  dispatchMessage,
  dispatchToast,
  getBusy,
  getToast,
  setBusyState,
  subscribe,
} from "./bus";
import type { Message } from "./types";

describe("message bus", () => {
  beforeEach(() => {
    dismissToast();
    setBusyState(false);
  });

  afterEach(() => {
    vi.useRealTimers();
    dismissToast();
    setBusyState(false);
  });

  it("stores and returns the current toast", () => {
    dispatchToast({ kind: "success", text: "Saved" });
    expect(getToast()?.text).toBe("Saved");
  });

  it("notifies subscribers when toast changes", () => {
    const listener = vi.fn();
    const unsubscribe = subscribe(listener);
    dispatchToast({ kind: "info", text: "Working" });
    expect(listener).toHaveBeenCalled();
    unsubscribe();
  });

  it("auto-dismisses timed toasts", () => {
    vi.useFakeTimers();
    dispatchToast({ kind: "success", text: "Done", duration_ms: 2000 });
    expect(getToast()?.text).toBe("Done");
    vi.advanceTimersByTime(2000);
    expect(getToast()).toBeNull();
  });

  it("keeps sticky errors until dismissed", () => {
    vi.useFakeTimers();
    dispatchToast({ kind: "error", text: "disk full", duration_ms: 0 });
    vi.advanceTimersByTime(60_000);
    expect(getToast()?.text).toBe("disk full");
    dismissToast();
    expect(getToast()).toBeNull();
  });

  it("tracks busy state without redundant notifications", () => {
    const listener = vi.fn();
    const unsubscribe = subscribe(listener);
    setBusyState(true);
    setBusyState(true);
    setBusyState(false);
    expect(getBusy()).toBe(false);
    expect(listener).toHaveBeenCalledTimes(2);
    unsubscribe();
  });

  it("replaces an existing toast timer when dispatching again", () => {
    vi.useFakeTimers();
    const base: Message = {
      id: "toast-1",
      kind: "success",
      text: "First",
      duration_ms: 5000,
      created_at: 0,
    };
    dispatchMessage(base);
    dispatchMessage({ ...base, text: "Second" });
    expect(getToast()?.text).toBe("Second");
    vi.advanceTimersByTime(5000);
    expect(getToast()).toBeNull();
  });

  it("dismisses only the matching toast id", () => {
    dispatchToast({ kind: "info", text: "Working", duration_ms: 0 });
    const id = getToast()?.id;
    dismissToast("other-id");
    expect(getToast()?.text).toBe("Working");
    dismissToast(id);
    expect(getToast()).toBeNull();
  });
});
