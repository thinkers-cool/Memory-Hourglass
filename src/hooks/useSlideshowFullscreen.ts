import { useEffect, useRef } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  enterWindowFullscreen,
  exitWindowFullscreen,
  isWindowFullscreen,
} from "../lib/windowFullscreen";

export function useSlideshowFullscreen(onSystemExit: () => void) {
  const onSystemExitRef = useRef(onSystemExit);
  onSystemExitRef.current = onSystemExit;

  useEffect(() => {
    let disposed = false;
    let armed = false;
    let exited = false;
    let unlistenResize: (() => void) | undefined;

    const handleSystemExit = async () => {
      if (!armed || exited || disposed) return;
      const fullscreen = await isWindowFullscreen();
      if (fullscreen) return;
      exited = true;
      onSystemExitRef.current();
    };

    const setup = async () => {
      await enterWindowFullscreen();
      if (disposed) return;
      armed = true;

      if (isTauri()) {
        unlistenResize = await getCurrentWindow().onResized(() => {
          void handleSystemExit();
        });
        return;
      }

      const onFullscreenChange = () => {
        void handleSystemExit();
      };
      document.addEventListener("fullscreenchange", onFullscreenChange);
      unlistenResize = () => {
        document.removeEventListener("fullscreenchange", onFullscreenChange);
      };
    };

    void setup();

    return () => {
      disposed = true;
      unlistenResize?.();
      void exitWindowFullscreen();
    };
  }, []);
}
