import { describe, expect, it, vi } from "vitest";
import i18n from "../i18n";
import {
  parseAppError,
  userFacingErrorMessage,
  errorNotification,
} from "./appError";

describe("parseAppError", () => {
  it("parses structured workspace payload", () => {
    const payload = parseAppError(
      JSON.stringify({
        code: "workspace_not_a_workspace",
        message: "not a workspace",
      }),
    );
    expect(payload.code).toBe("workspace_not_a_workspace");
    expect(userFacingErrorMessage(payload)).toContain("not a workspace");
  });

  it("parses structured invoke error objects", () => {
    const payload = parseAppError({
      code: "library",
      message: "SMB server did not respond in time; check the host and network",
    });
    expect(payload.code).toBe("library");
    expect(userFacingErrorMessage(payload)).toContain("did not respond");
  });

  it("uses a generic workspace code for unstructured errors", () => {
    const payload = parseAppError("unexpected failure");
    expect(payload.code).toBe("workspace");
    expect(payload.message).toBe("unexpected failure");
  });

  it("handles malformed json and empty errors", () => {
    expect(parseAppError("{bad").code).toBe("workspace");
    expect(parseAppError(null).message.length).toBeGreaterThan(0);
  });

  it("builds error notifications", () => {
    const notification = errorNotification({
      code: "library",
      message: "disk full",
    });
    expect(notification.kind).toBe("error");
    expect(notification.text).toBe("disk full");
  });

  it("extracts messages from Error instances", () => {
    const payload = parseAppError(new Error("network down"));
    expect(payload.message).toBe("network down");
    expect(payload.code).toBe("workspace");
  });

  it("stringifies non-string unknown errors", () => {
    expect(parseAppError(404).message).toBe("404");
  });

  it("prefers translated workspace messages", () => {
    const payload = parseAppError({
      code: "workspace_not_a_workspace",
      message: "The selected folder is not a Memory Hourglass workspace.",
    });
    expect(userFacingErrorMessage(payload)).toContain("workspace");
  });

  it("returns translated message when payload message is empty", () => {
    const payload = parseAppError({ code: "not_found", message: "" });
    expect(userFacingErrorMessage(payload).length).toBeGreaterThan(0);
  });

  it("returns payload message when no translation exists", () => {
    const payload = parseAppError({
      code: "library",
      message: "custom backend detail",
    });
    expect(userFacingErrorMessage(payload)).toBe("custom backend detail");
  });

  it("returns generic message when payload is empty and untranslated", () => {
    vi.spyOn(i18n, "exists").mockReturnValue(false);
    expect(userFacingErrorMessage({ code: "catalog", message: "" })).toBe(
      i18n.t("errors:generic.message"),
    );
    vi.restoreAllMocks();
  });

  it("returns translated text when payload message matches translation", () => {
    const payload = parseAppError({
      code: "not_found",
      message: "The requested item was not found.",
    });
    expect(userFacingErrorMessage(payload)).toBe(
      "The requested item was not found.",
    );
  });

  it("returns raw detail for non-workspace codes with translation", () => {
    const payload = parseAppError({
      code: "library",
      message: "Host unreachable",
    });
    expect(userFacingErrorMessage(payload)).toBe("Host unreachable");
  });

  it("uses generic fallback for empty unstructured errors", () => {
    expect(parseAppError("   ").message).toBe(
      i18n.t("errors:generic.fallback"),
    );
  });

  it("returns translated message for workspace codes with custom detail", () => {
    const payload = parseAppError({
      code: "workspace_path_not_found",
      message: "custom detail",
    });
    expect(userFacingErrorMessage(payload)).not.toBe("custom detail");
    expect(userFacingErrorMessage(payload).length).toBeGreaterThan(0);
  });

  it("returns payload message when translation key is missing", () => {
    const originalExists = i18n.exists.bind(i18n);
    vi.spyOn(i18n, "exists").mockImplementation((key) => {
      if (key === "custom_code.message") return false;
      return originalExists(key as never);
    });
    expect(
      userFacingErrorMessage({
        code: "custom_code",
        message: "backend detail",
      }),
    ).toBe("backend detail");
    vi.restoreAllMocks();
  });

  it("rejects malformed structured invoke payloads", () => {
    expect(parseAppError({ code: "library" }).code).toBe("workspace");
    expect(parseAppError(JSON.stringify({ code: 1, message: "x" })).code).toBe(
      "workspace",
    );
  });
});
