import { useCallback, useEffect, useState } from "react";
import {
  DEFAULT_PANEL_ORDER_STORE,
  loadPanelOrderStore,
  type PanelOrderStore,
  type PanelSectionKey,
  savePanelOrderStore,
  togglePanelOrderMode,
} from "../lib/panelOrder";

export function usePanelOrder(workspaceId: string) {
  const [store, setStore] = useState<PanelOrderStore>(() =>
    loadPanelOrderStore(workspaceId),
  );

  useEffect(() => {
    setStore(loadPanelOrderStore(workspaceId));
  }, [workspaceId]);

  const toggleSection = useCallback(
    (section: PanelSectionKey) => {
      const next = {
        ...store,
        [section]: togglePanelOrderMode(store[section]),
      };
      setStore(next);
      savePanelOrderStore(workspaceId, next);
    },
    [store, workspaceId],
  );

  return { store, toggleSection };
}

export { DEFAULT_PANEL_ORDER_STORE };
