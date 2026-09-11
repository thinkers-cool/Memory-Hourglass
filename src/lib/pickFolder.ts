import { isTauri } from "@tauri-apps/api/core";
import { homeDir } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import i18n from "../i18n";

export type PickFolderOptions = {
  title?: string;
  createDirectory?: boolean;
};

export async function pickFolder(
  options: PickFolderOptions = {},
): Promise<string | null> {
  if (!isTauri()) {
    throw new Error(i18n.t("errors:notDesktopApp"));
  }

  const defaultPath = await homeDir().catch(() => undefined);

  return open({
    title: options.title ?? i18n.t("common:folderPicker.default"),
    directory: true,
    multiple: false,
    recursive: true,
    defaultPath,
    canCreateDirectories: options.createDirectory ?? false,
  });
}

export function formatError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}
