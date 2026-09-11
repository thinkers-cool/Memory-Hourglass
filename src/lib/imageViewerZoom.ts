export const IMAGE_VIEWER_ZOOM_MIN = 0.25;
export const IMAGE_VIEWER_ZOOM_MAX = 8;

export function formatImageViewerZoom(scale: number) {
  return `${Math.round(scale * 100)}%`;
}

export type ImageViewerZoomActions = {
  zoomIn: () => void;
  zoomOut: () => void;
  resetTransform: () => void;
};

export function handleImageViewerZoomKey(
  event: Pick<KeyboardEvent, "key" | "preventDefault">,
  actions: ImageViewerZoomActions,
): boolean {
  if (event.key === "=" || event.key === "+") {
    event.preventDefault();
    actions.zoomIn();
    return true;
  }
  if (event.key === "-") {
    event.preventDefault();
    actions.zoomOut();
    return true;
  }
  if (event.key === "0") {
    event.preventDefault();
    actions.resetTransform();
    return true;
  }
  return false;
}
