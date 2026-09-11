import {
  TransformComponent,
  TransformWrapper,
  type ReactZoomPanPinchContentRef,
} from "react-zoom-pan-pinch";
import type { ZoomableMediaOnTransform } from "../../hooks/useZoomableMedia";
import {
  IMAGE_VIEWER_ZOOM_MAX,
  IMAGE_VIEWER_ZOOM_MIN,
} from "../../lib/imageViewerZoom";

export function ZoomableMedia({
  resetKey,
  className = "relative flex min-h-0 flex-1 flex-col",
  transformRef,
  onTransform,
  children,
}: {
  resetKey: string | number;
  className?: string;
  transformRef: React.RefObject<ReactZoomPanPinchContentRef | null>;
  onTransform: ZoomableMediaOnTransform;
  children: React.ReactNode;
}) {
  return (
    <div className={className}>
      <TransformWrapper
        key={resetKey}
        ref={transformRef}
        minScale={IMAGE_VIEWER_ZOOM_MIN}
        maxScale={IMAGE_VIEWER_ZOOM_MAX}
        initialScale={1}
        centerOnInit
        centerZoomedOut
        onTransform={onTransform}
      >
        <TransformComponent wrapperClass="!h-full !w-full" contentClass="!p-2">
          {children}
        </TransformComponent>
      </TransformWrapper>
    </div>
  );
}
