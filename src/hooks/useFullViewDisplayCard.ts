import { useEffect, useRef, useState } from "react";
import type { AssetCard } from "../types";
import {
  prefetchFullViewNeighbors,
  preloadFullViewImage,
} from "../lib/fullViewMedia";

export function useFullViewDisplayCard(
  card: AssetCard,
  neighborCards: AssetCard[],
): AssetCard {
  const [displayCard, setDisplayCard] = useState(card);
  const requestIdRef = useRef(card.id);

  useEffect(() => {
    requestIdRef.current = card.id;
    prefetchFullViewNeighbors(neighborCards);

    if (card.kind === "video") {
      setDisplayCard(card);
      return;
    }

    let cancelled = false;
    void preloadFullViewImage(card.abs_path)
      .then(() => {
        if (!cancelled && requestIdRef.current === card.id) {
          setDisplayCard(card);
        }
      })
      .catch(() => {
        if (!cancelled && requestIdRef.current === card.id) {
          setDisplayCard(card);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [card, neighborCards]);

  useEffect(() => {
    if (displayCard.id === card.id) {
      setDisplayCard(card);
    }
  }, [card, displayCard.id]);

  return displayCard;
}
