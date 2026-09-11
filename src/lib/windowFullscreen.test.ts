import { beforeEach, describe, expect, it, vi } from "vitest";

const { isTauri, setFullscreen, isFullscreen } = vi.hoisted(() => ({
  isTauri: vi.fn(() => false),
  setFullscreen: vi.fn(async () => undefined),
  isFullscreen: vi.fn(async () => false),
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    setFullscreen,
    isFullscreen,
  }),
}));

import {
  enterWindowFullscreen,
  exitWindowFullscreen,
  isWindowFullscreen,
} from "./windowFullscreen";

describe("windowFullscreen", () => {
  beforeEach(() => {
    isTauri.mockReset();
    setFullscreen.mockClear();
    isFullscreen.mockClear();
  });

  it("uses browser fullscreen when not in tauri", async () => {
    const requestFullscreen = vi.fn(async () => undefined);
    Object.defineProperty(document, "fullscreenElement", {
      configurable: true,
      value: null,
    });
    document.documentElement.requestFullscreen = requestFullscreen;

    await enterWindowFullscreen();
    expect(requestFullscreen).toHaveBeenCalled();
  });

  it("uses tauri fullscreen in desktop app", async () => {
    isTauri.mockReturnValue(true);

    await enterWindowFullscreen();
    expect(setFullscreen).toHaveBeenCalledWith(true);

    await exitWindowFullscreen();
    expect(setFullscreen).toHaveBeenCalledWith(false);
  });

  it("reads tauri fullscreen state", async () => {
    isTauri.mockReturnValue(true);
    isFullscreen.mockResolvedValue(true);
    await expect(isWindowFullscreen()).resolves.toBe(true);
  });

  it("uses browser exit fullscreen when element is active", async () => {
    const exitFullscreen = vi.fn(async () => undefined);
    document.exitFullscreen = exitFullscreen;
    Object.defineProperty(document, "fullscreenElement", {
      configurable: true,
      value: document.documentElement,
    });

    await exitWindowFullscreen();
    expect(exitFullscreen).toHaveBeenCalled();
  });

  it("skips browser enter when already fullscreen or unsupported", async () => {
    const requestFullscreen = vi.fn(async () => undefined);
    document.documentElement.requestFullscreen = requestFullscreen;
    Object.defineProperty(document, "fullscreenElement", {
      configurable: true,
      value: document.documentElement,
    });

    await enterWindowFullscreen();
    expect(requestFullscreen).not.toHaveBeenCalled();

    Object.defineProperty(document, "fullscreenElement", {
      configurable: true,
      value: null,
    });
    document.documentElement.requestFullscreen = undefined as never;
    await enterWindowFullscreen();
  });

  it("reads browser fullscreen state", async () => {
    Object.defineProperty(document, "fullscreenElement", {
      configurable: true,
      value: document.documentElement,
    });
    await expect(isWindowFullscreen()).resolves.toBe(true);
  });
});
