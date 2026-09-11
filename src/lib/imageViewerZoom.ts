export const IMAGE_VIEWER_ZOOM_MIN = 0.25;
export const IMAGE_VIEWER_ZOOM_MAX = 8;

export function formatImageViewerZoom(scale: number) {
  return `${Math.round(scale * 100)}%`;
}
