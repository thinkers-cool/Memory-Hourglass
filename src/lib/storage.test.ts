import { describe, expect, it, vi } from "vitest";
import { saveStoredJson } from "./storage";

describe("saveStoredJson", () => {
  it("ignores storage write failures", () => {
    const setItem = vi.spyOn(localStorage, "setItem").mockImplementation(() => {
      throw new Error("quota");
    });
    expect(() => saveStoredJson("key", { ok: true })).not.toThrow();
    setItem.mockRestore();
  });
});
