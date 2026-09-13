import { isTauri } from "@tauri-apps/api/core";
import { homeDir } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import i18n from "../i18n";

export type PickFolderOptions = {
  title?: string;
  createDirectory?: boolean;
  multiple?: boolean;
};

export async function openFolderDialog(
  options: PickFolderOptions,
): Promise<string | string[] | null> {
  if (!isTauri()) {
    throw new Error(i18n.t("errors:notDesktopApp"));
  }

  const defaultPath = await homeDir().catch(() => undefined);

  return open({
    title: options.title ?? i18n.t("common:folderPicker.default"),
    directory: true,
    multiple: options.multiple ?? false,
    recursive: true,
    defaultPath,
    canCreateDirectories: options.createDirectory ?? false,
  });
}

export async function pickFolder(
  options: PickFolderOptions = {},
): Promise<string | null> {
  const result = await openFolderDialog({ ...options, multiple: false });
  if (result === null) return null;
  if (Array.isArray(result)) {
    return result[0] ?? null;
  }
  return result;
}

export async function pickFolders(
  options: PickFolderOptions = {},
): Promise<string[]> {
  const result = await openFolderDialog({ ...options, multiple: true });
  if (result === null) return [];
  if (Array.isArray(result)) return result;
  return [result];
}

export function formatError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}
