import { useMemo, useRef, useState } from "react";
import type { AssetCard, AssetDetail } from "../../types";

export function useLibrarySelection() {
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [detail, setDetail] = useState<AssetDetail | null>(null);
  const [compareOpen, setCompareOpen] = useState(false);
  const [compareIds, setCompareIds] = useState<number[]>([]);
  const [compareItems, setCompareItems] = useState<AssetCard[]>([]);
  const [compareDetails, setCompareDetails] = useState<
    Record<number, { tag_ids: number[]; album_ids: number[] }>
  >({});

  const lastSelectedIndexRef = useRef<number | null>(null);
  const selectedIdRef = useRef<number | null>(null);
  selectedIdRef.current = selectedId;

  const selectedList = useMemo(() => Array.from(selectedIds), [selectedIds]);

  return {
    selectedId,
    setSelectedId,
    selectedIds,
    setSelectedIds,
    detail,
    setDetail,
    compareOpen,
    setCompareOpen,
    compareIds,
    setCompareIds,
    compareItems,
    setCompareItems,
    compareDetails,
    setCompareDetails,
    lastSelectedIndexRef,
    selectedIdRef,
    selectedList,
  };
}
