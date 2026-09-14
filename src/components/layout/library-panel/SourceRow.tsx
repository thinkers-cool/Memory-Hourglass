import { useTranslation } from "react-i18next";
import type { RootStats } from "../../../types";
import type { ScanProgress } from "../../../lib/scanStatus";
import { Folder, FolderInput, Network, RefreshCw, Trash2 } from "lucide-react";
import { ItemCountSubtitle } from "./ItemCountSubtitle";
import { PanelListRow, PanelRowActionButton } from "./PanelListRow";
import { SourceScanProgress } from "./SourceScanProgress";
import { iconClass, rootDisplayName } from "./panelStyles";

export function SourceRow({
  root,
  active,
  onSelect,
  onSync,
  onRelink,
  onRemove,
  scanProgress,
}: {
  root: RootStats;
  active?: boolean;
  scanProgress?: ScanProgress | null;
  onSelect: () => void;
  onSync: () => void;
  onRelink: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation(["library", "common"]);
  const name = rootDisplayName(root.path);
  const offline = root.status === "offline";
  const scanning = scanProgress != null;
  const isSmb = root.kind === "smb";
  const Icon = isSmb ? Network : Folder;

  return (
    <PanelListRow
      active={active}
      dimmed={offline}
      badge={<Icon className="h-4 w-4" aria-hidden />}
      badgeTitle={
        isSmb ? t("panel.badge.networkSource") : t("panel.badge.localFolder")
      }
      title={name}
      titleTooltip={root.path}
      subtitle={
        <>
          {offline ? (
            <span className="font-medium text-warning">
              {t("panel.offline")}
            </span>
          ) : null}
          {scanning ? <SourceScanProgress progress={scanProgress!} /> : null}
          <ItemCountSubtitle count={root.asset_count} />
        </>
      }
      onClick={onSelect}
      actions={
        <>
          <PanelRowActionButton title={t("panel.sync")} onClick={onSync}>
            <RefreshCw className={iconClass} />
          </PanelRowActionButton>
          <PanelRowActionButton title={t("panel.relink")} onClick={onRelink}>
            <FolderInput className={iconClass} />
          </PanelRowActionButton>
          <PanelRowActionButton
            title={t("common:action.remove")}
            onClick={onRemove}
          >
            <Trash2 className={iconClass} />
          </PanelRowActionButton>
        </>
      }
    />
  );
}
