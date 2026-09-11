import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createLocalStorageMock } from "../../test/storageMock";
import {
  DEFAULT_SLIDESHOW_SETTINGS,
  loadSlideshowSettings,
  saveSlideshowSettings,
} from "./settings";

describe("loadSlideshowSettings", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", createLocalStorageMock());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns defaults when storage is empty", () => {
    expect(loadSlideshowSettings()).toEqual(DEFAULT_SLIDESHOW_SETTINGS);
  });

  it("uses default interval when stored interval is omitted", () => {
    localStorage.setItem(
      "memhg.slideshow.settings",
      JSON.stringify({ shuffle: true }),
    );
    expect(loadSlideshowSettings()).toEqual({
      ...DEFAULT_SLIDESHOW_SETTINGS,
      shuffle: true,
    });
  });

  it("merges stored values with defaults", () => {
    localStorage.setItem(
      "memhg.slideshow.settings",
      JSON.stringify({ shuffle: true, intervalMs: 5000 }),
    );
    expect(loadSlideshowSettings()).toEqual({
      ...DEFAULT_SLIDESHOW_SETTINGS,
      shuffle: true,
      intervalMs: 5000,
    });
  });

  it("returns defaults when storage is invalid json", () => {
    localStorage.setItem("memhg.slideshow.settings", "{bad json");
    expect(loadSlideshowSettings()).toEqual(DEFAULT_SLIDESHOW_SETTINGS);
  });

  it("returns defaults when stored value is not an object", () => {
    localStorage.setItem("memhg.slideshow.settings", "[]");
    expect(loadSlideshowSettings()).toEqual(DEFAULT_SLIDESHOW_SETTINGS);
  });

  it("coerces invalid stored interval values", () => {
    localStorage.setItem(
      "memhg.slideshow.settings",
      JSON.stringify({ intervalMs: 9999, shuffle: true }),
    );
    expect(loadSlideshowSettings()).toEqual({
      ...DEFAULT_SLIDESHOW_SETTINGS,
      shuffle: true,
      intervalMs: 3000,
    });
  });
});

describe("saveSlideshowSettings", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", createLocalStorageMock());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("persists settings as json", () => {
    saveSlideshowSettings({
      theme: "push",
      intervalMs: 2000,
      loop: false,
      shuffle: true,
      muteVideos: false,
    });
    expect(
      JSON.parse(localStorage.getItem("memhg.slideshow.settings")!),
    ).toEqual({
      theme: "push",
      intervalMs: 2000,
      loop: false,
      shuffle: true,
      muteVideos: false,
    });
  });
});
