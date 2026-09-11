import { describe, expect, it } from "vitest";
import {
  errorNotification,
  formatErrorMessage,
  infoNotification,
  isErrorNotification,
  notificationKind,
  notificationText,
  successNotification,
} from "./notification";

describe("notification", () => {
  it("formats errors", () => {
    expect(formatErrorMessage(new Error("boom"))).toBe("boom");
    expect(formatErrorMessage("plain")).toBe("plain");
  });

  it("builds notification payloads", () => {
    expect(successNotification("ok")).toEqual({ kind: "success", text: "ok" });
    expect(infoNotification("note")).toEqual({ kind: "info", text: "note" });
    expect(errorNotification(new Error("x")).kind).toBe("error");
  });

  it("reads notification helpers", () => {
    expect(notificationText(null)).toBe("");
    expect(notificationText({ kind: "info", text: "  hi  " })).toBe("hi");
    expect(isErrorNotification({ kind: "error", text: "x" })).toBe(true);
    expect(isErrorNotification({ kind: "info", text: "x" })).toBe(false);
    expect(notificationKind({ kind: "success", text: "x" })).toBe("success");
    expect(notificationKind(null)).toBeNull();
  });
});
