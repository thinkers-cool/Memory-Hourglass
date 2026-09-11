import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export async function enterWindowFullscreen(): Promise<void> {
  if (isTauri()) {
    await getCurrentWindow().setFullscreen(true);
    return;
  }
  if (
    !document.fullscreenElement &&
    document.documentElement.requestFullscreen
  ) {
    await document.documentElement.requestFullscreen();
  }
}

export async function exitWindowFullscreen(): Promise<void> {
  if (isTauri()) {
    await getCurrentWindow().setFullscreen(false);
    return;
  }
  if (document.fullscreenElement && document.exitFullscreen) {
    await document.exitFullscreen();
  }
}

export async function isWindowFullscreen(): Promise<boolean> {
  if (isTauri()) {
    return getCurrentWindow().isFullscreen();
  }
  return document.fullscreenElement !== null;
}
