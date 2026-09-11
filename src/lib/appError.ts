import type { Notification } from "../types";
import i18n from "../i18n";

export type ErrorCode =
  | "catalog"
  | "library"
  | "scan"
  | "metadata"
  | "query"
  | "export"
  | "job_busy"
  | "conflict"
  | "not_found"
  | "invalid_input"
  | "io"
  | "sql"
  | "migrate"
  | "json"
  | "workspace_path_not_found"
  | "workspace_not_a_workspace"
  | "workspace_already_exists"
  | "workspace_not_empty"
  | "workspace_not_directory"
  | "workspace_corrupt"
  | "workspace_not_open"
  | "workspace";

export type AppErrorPayload = {
  code: ErrorCode;
  message: string;
};

function extractRaw(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return String(error);
}

function isAppErrorPayload(value: unknown): value is AppErrorPayload {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const record = value as Record<string, unknown>;
  return typeof record.code === "string" && typeof record.message === "string";
}

export function parseAppError(error: unknown): AppErrorPayload {
  if (isAppErrorPayload(error)) {
    return error;
  }

  const raw = extractRaw(error).trim();
  if (raw.startsWith("{")) {
    try {
      const parsed: unknown = JSON.parse(raw);
      if (isAppErrorPayload(parsed)) {
        return parsed;
      }
    } catch {
      // ignore malformed JSON
    }
  }

  return {
    code: "workspace",
    message: raw || i18n.t("errors:generic.fallback"),
  };
}

export function userFacingErrorMessage(payload: AppErrorPayload): string {
  const key = `${payload.code}.message`;
  const hasTranslation = i18n.exists(key, { ns: "errors" });
  const translated = hasTranslation ? i18n.t(key, { ns: "errors" }) : "";

  if (payload.message) {
    if (!hasTranslation) return payload.message;
    if (payload.message === translated) return translated;
    if (payload.code.startsWith("workspace_")) return translated;
    return payload.message;
  }

  if (hasTranslation) return translated;
  return i18n.t("errors:generic.message");
}

export function errorNotification(error: unknown): Notification {
  const payload = parseAppError(error);
  return {
    kind: "error",
    text: userFacingErrorMessage(payload),
  };
}
