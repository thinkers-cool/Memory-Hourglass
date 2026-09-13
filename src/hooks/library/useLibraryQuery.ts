import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { pickActiveScanStatus } from "../../lib/scanStatusSelection";
import * as api from "../../api/client";
import { mergeLibraryFilter } from "../../lib/libraryFilters";
import type { FilterBarState } from "../../lib/libraryActions";
import { errorNotification } from "../../lib/notification";
import { patchAssetThumbs } from "../../lib/patchAssetThumbs";
import { DEFAULT_SORT_DIR, encodeSortParam } from "../../lib/sortSettings";
import type {
  Album,
  AssetCard,
  AssetDetail,
  AssetFilter,
  Notification,
  RootStats,
  SmartCollection,
  ScanThumbUpdate,
  TagDto,
} from "../../types";
import { PAGE } from "./constants";

const GRID_REFRESH_MS = 8000;
const META_REFRESH_MS = 5000;

function shouldScheduleGridRefresh(stage: string, indexed: number): boolean {
  return (
    indexed > 0 && (stage === "cataloging" || stage === "indexing")
  );
}

export function useLibraryQuery(
  selectedIdRef: React.MutableRefObject<number | null>,
  setDetail: React.Dispatch<React.SetStateAction<AssetDetail | null>>,
  setNotification: React.Dispatch<React.SetStateAction<Notification | null>>,
) {
  const [roots, setRoots] = useState<RootStats[]>([]);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [collections, setCollections] = useState<SmartCollection[]>([]);
  const [tags, setTags] = useState<TagDto[]>([]);
  const [deletedCount, setDeletedCount] = useState(0);
  const [items, setItems] = useState<AssetCard[]>([]);
  const [total, setTotal] = useState(0);
  const [extraFilter, setExtraFilter] = useState<AssetFilter>({});
  const [filterBar, setFilterBar] = useState<FilterBarState>({
    ratingMin: "",
    syncStates: [],
    deleteStatus: "",
    camera: "",
    tagIds: [],
    albumIds: [],
    metaSearch: "",
    hasGps: false,
    hasDuplicate: false,
    captureFrom: "",
    captureTo: "",
    sort: "date",
    sortDir: DEFAULT_SORT_DIR.date,
  });
  const [selectedCollectionId, setSelectedCollectionId] = useState<
    number | null
  >(null);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const [scanStatusByRoot, setScanStatusByRoot] = useState<
    Record<number, string>
  >({});
  const [focusScanRootId, setFocusScanRootId] = useState<number | null>(null);

  const scanStatus = useMemo(
    () => pickActiveScanStatus(scanStatusByRoot),
    [scanStatusByRoot],
  );

  const offsetRef = useRef(0);
  const filterRef = useRef<AssetFilter>({});
  const requestGenerationRef = useRef(0);

  const filter = useMemo(
    () => mergeLibraryFilter(filterBar, extraFilter),
    [filterBar, extraFilter],
  );

  filterRef.current = filter;

  const sortParam = useMemo(
    () => encodeSortParam(filterBar.sort, filterBar.sortDir),
    [filterBar.sort, filterBar.sortDir],
  );

  const refreshMeta = useCallback(async () => {
    try {
      setRoots(await api.listRootStats());
      setAlbums(await api.listAlbums());
      setCollections(await api.listSmartCollections());
      setTags(await api.listTags());
      setDeletedCount(await api.countAssets({ deleted_only: true }));
    } catch (error) {
      setNotification(errorNotification(error));
    }
  }, [setNotification]);

  const refreshGrid = useCallback(async () => {
    const generation = ++requestGenerationRef.current;
    offsetRef.current = 0;
    try {
      const result = await api.queryAssets(
        filterRef.current,
        sortParam,
        0,
        PAGE,
      );
      if (generation !== requestGenerationRef.current) {
        return;
      }
      setItems(result.items);
      setTotal(result.total);
      setHasMore(result.items.length < result.total);
    } catch (error) {
      if (generation !== requestGenerationRef.current) {
        return;
      }
      setNotification(errorNotification(error));
    }
  }, [setNotification, sortParam]);

  const refreshAll = useCallback(async () => {
    await refreshMeta();
    await refreshGrid();
  }, [refreshMeta, refreshGrid]);

  const applyThumbUpdates = useCallback((thumbs: ScanThumbUpdate[]) => {
    if (thumbs.length === 0) {
      return;
    }
    setItems((prev) => patchAssetThumbs(prev, thumbs));
  }, []);

  useEffect(() => {
    void refreshMeta().then(() => {
      void api.resumePendingScans().catch(console.error);
    });
  }, [refreshMeta]);

  useEffect(() => {
    void refreshGrid();
  }, [filter, sortParam, refreshGrid]);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenThumbs: (() => void) | undefined;
    let metaRefreshTimer: number | undefined;
    let gridRefreshTimer: number | undefined;
    let lastStatusUpdate = 0;

    void api
      .onScanProgress((progress) => {
        const now = Date.now();
        if (
          progress.stage === "done" ||
          progress.stage === "error" ||
          now - lastStatusUpdate >= 500
        ) {
          lastStatusUpdate = now;
          setScanStatusByRoot((prev) => ({
            ...prev,
            [progress.root_id]: `${progress.stage}: ${progress.indexed}/${progress.scanned}`,
          }));
        }

        if (progress.stage === "done" || progress.stage === "error") {
          setFocusScanRootId((prev) =>
            prev === progress.root_id ? null : prev,
          );
          setScanStatusByRoot((prev) => {
            const next = { ...prev };
            delete next[progress.root_id];
            return next;
          });
          window.clearTimeout(metaRefreshTimer);
          window.clearTimeout(gridRefreshTimer);
          void refreshAll();
          const currentSelectedId = selectedIdRef.current;
          if (currentSelectedId !== null) {
            void api
              .getAsset(currentSelectedId)
              .then(setDetail)
              .catch(console.error);
          }
          return;
        }

        if (shouldScheduleGridRefresh(progress.stage, progress.indexed)) {
          window.clearTimeout(gridRefreshTimer);
          gridRefreshTimer = window.setTimeout(() => {
            void refreshGrid();
          }, GRID_REFRESH_MS);
        }

        if (progress.stage === "cataloging" || progress.stage === "indexing") {
          window.clearTimeout(metaRefreshTimer);
          metaRefreshTimer = window.setTimeout(() => {
            void refreshMeta();
          }, META_REFRESH_MS);
        }
      })
      .then((fn) => {
        unlistenProgress = fn;
      })
      .catch(console.error);

    void api
      .onScanThumbs((event) => {
        applyThumbUpdates(event.thumbs);
      })
      .then((fn) => {
        unlistenThumbs = fn;
      })
      .catch(console.error);

    return () => {
      unlistenProgress?.();
      unlistenThumbs?.();
      window.clearTimeout(metaRefreshTimer);
      window.clearTimeout(gridRefreshTimer);
    };
  }, [
    applyThumbUpdates,
    refreshAll,
    refreshGrid,
    refreshMeta,
    selectedIdRef,
    setDetail,
  ]);

  const clearScanStatus = useCallback(() => {
    setScanStatusByRoot({});
  }, []);

  const loadMore = useCallback(async () => {
    if (loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const nextOffset = offsetRef.current + PAGE;
      const result = await api.queryAssets(
        filterRef.current,
        sortParam,
        nextOffset,
        PAGE,
      );
      offsetRef.current = nextOffset;
      setItems((prev) => {
        const seen = new Set(prev.map((item) => item.id));
        const nextItems = result.items.filter((item) => !seen.has(item.id));
        return [...prev, ...nextItems];
      });
      setTotal(result.total);
      setHasMore(nextOffset + result.items.length < result.total);
    } catch (error) {
      setNotification(errorNotification(error));
    } finally {
      setLoadingMore(false);
    }
  }, [sortParam, hasMore, loadingMore, setNotification]);

  return {
    roots,
    albums,
    collections,
    tags,
    deletedCount,
    items,
    setItems,
    total,
    extraFilter,
    setExtraFilter,
    filterBar,
    setFilterBar,
    selectedCollectionId,
    setSelectedCollectionId,
    loadingMore,
    hasMore,
    scanStatus,
    scanStatusByRoot,
    focusScanRootId,
    setFocusScanRootId,
    clearScanStatus,
    filterRef,
    sortParam,
    refreshMeta,
    refreshGrid,
    refreshAll,
    loadMore,
  };
}
