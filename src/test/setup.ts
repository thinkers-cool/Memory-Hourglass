import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";
import "../i18n";

const localStorageStore: Record<string, string> = {};

function clearLocalStorageStore() {
  for (const key of Object.keys(localStorageStore)) {
    delete localStorageStore[key];
  }
}

const localStorageMock: Storage = {
  get length() {
    return Object.keys(localStorageStore).length;
  },
  key(index: number) {
    return Object.keys(localStorageStore)[index] ?? null;
  },
  getItem(key: string) {
    return localStorageStore[key] ?? null;
  },
  setItem(key: string, value: string) {
    localStorageStore[key] = value;
  },
  removeItem(key: string) {
    delete localStorageStore[key];
  },
  clear() {
    clearLocalStorageStore();
  },
};

class MockResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

function restoreTestGlobals() {
  vi.stubGlobal("localStorage", localStorageMock);
  vi.stubGlobal("ResizeObserver", MockResizeObserver);
}

restoreTestGlobals();

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.clearAllTimers();
  clearLocalStorageStore();
  vi.clearAllMocks();
  restoreTestGlobals();
});

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  isTauri: () => false,
  invoke: vi.fn(),
}));

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

Object.defineProperty(HTMLMediaElement.prototype, "play", {
  configurable: true,
  value: vi.fn().mockResolvedValue(undefined),
});

Object.defineProperty(HTMLMediaElement.prototype, "pause", {
  configurable: true,
  value: vi.fn(),
});
