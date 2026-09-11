import type { Notification, NotificationKind } from "../types";
import { errorNotification as appErrorNotification } from "./appError";

export function formatErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

export function errorNotification(error: unknown): Notification {
  return appErrorNotification(error);
}

export function successNotification(text: string): Notification {
  return { kind: "success", text };
}

export function infoNotification(text: string): Notification {
  return { kind: "info", text };
}

export function notificationText(notification: Notification | null): string {
  return notification?.text.trim() ?? "";
}

export function isErrorNotification(
  notification: Notification | null,
): boolean {
  return notification?.kind === "error";
}

export function notificationKind(
  notification: Notification | null,
): NotificationKind | null {
  return notification?.kind ?? null;
}
