import { useCallback, useEffect, useMemo, useState } from "react";
import type { Album, TagDto } from "../../types";
import {
  EMPTY_STAMP_CONFIG,
  isStampConfigValid,
  pruneStampConfig,
  type StampConfig,
} from "../../lib/stamp";

export function useStamp(tags: TagDto[], albums: Album[]) {
  const [config, setConfig] = useState<StampConfig>(EMPTY_STAMP_CONFIG);
  const [armed, setArmed] = useState(false);
  const [matchedIds, setMatchedIds] = useState<Set<number>>(() => new Set());

  const tagIdSet = useMemo(() => new Set(tags.map((tag) => tag.id)), [tags]);
  const albumIdSet = useMemo(() => new Set(albums.map((album) => album.id)), [albums]);

  useEffect(() => {
    setConfig((prev) => pruneStampConfig(prev, tagIdSet, albumIdSet));
  }, [tagIdSet, albumIdSet]);

  useEffect(() => {
    setArmed(isStampConfigValid(config));
  }, [config]);

  const setStampRating = useCallback((rating: number | null) => {
    setConfig((prev) => ({ ...prev, rating }));
  }, []);

  const toggleStampTag = useCallback((tagId: number, add: boolean) => {
    setConfig((prev) => {
      const tagIds = add
        ? prev.tag_ids.includes(tagId)
          ? prev.tag_ids
          : [...prev.tag_ids, tagId]
        : prev.tag_ids.filter((id) => id !== tagId);
      return { ...prev, tag_ids: tagIds };
    });
  }, []);

  const toggleStampAlbum = useCallback((albumId: number, add: boolean) => {
    setConfig((prev) => {
      const albumIds = add
        ? prev.album_ids.includes(albumId)
          ? prev.album_ids
          : [...prev.album_ids, albumId]
        : prev.album_ids.filter((id) => id !== albumId);
      return { ...prev, album_ids: albumIds };
    });
  }, []);

  const disarm = useCallback(() => {
    setConfig(EMPTY_STAMP_CONFIG);
    setMatchedIds(new Set());
  }, []);

  const markStampResults = useCallback(
    (stampedIds: number[], unstampedIds: number[]) => {
      setMatchedIds((prev) => {
        const next = new Set(prev);
        for (const id of stampedIds) {
          next.add(id);
        }
        for (const id of unstampedIds) {
          next.delete(id);
        }
        return next;
      });
    },
    [],
  );

  const clearMatchedIds = useCallback(() => {
    setMatchedIds(new Set());
  }, []);

  return {
    config,
    armed,
    matchedIds,
    setStampRating,
    toggleStampTag,
    toggleStampAlbum,
    disarm,
    markStampResults,
    clearMatchedIds,
  };
}
