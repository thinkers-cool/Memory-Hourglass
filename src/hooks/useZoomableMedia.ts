import { useCallback, useRef, useState } from "react";
import type {
  ReactZoomPanPinchContentRef,
  ReactZoomPanPinchProps,
} from "react-zoom-pan-pinch";

export type ZoomableMediaOnTransform = NonNullable<
  ReactZoomPanPinchProps["onTransform"]
>;

export function useZoomableMedia() {
  const ref = useRef<ReactZoomPanPinchContentRef>(null);
  const [scale, setScale] = useState(1);

  const onTransform = useCallback<ZoomableMediaOnTransform>(
    (_instance, state) => {
      setScale(state.scale);
    },
    [],
  );

  const zoomIn = useCallback(() => {
    void ref.current?.zoomIn();
  }, []);

  const zoomOut = useCallback(() => {
    void ref.current?.zoomOut();
  }, []);

  const resetTransform = useCallback(() => {
    void ref.current?.resetTransform();
  }, []);

  return { ref, scale, onTransform, zoomIn, zoomOut, resetTransform };
}
