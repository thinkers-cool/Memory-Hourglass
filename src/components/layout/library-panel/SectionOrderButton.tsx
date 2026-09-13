import { useTranslation } from "react-i18next";
import { ArrowDownAZ, ArrowUpAZ } from "lucide-react";
import type { PanelOrderMode } from "../../../lib/panelOrder";
import { IconTooltip } from "../../shared/Tooltip";
import { iconButtonClass, iconClass } from "./panelStyles";

export function SectionOrderButton({
  mode,
  busy,
  onToggle,
}: {
  mode: PanelOrderMode;
  busy: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation("library");
  const ascending = mode === "name-asc";
  const tip = ascending ? t("panel.order.nameAsc") : t("panel.order.nameDesc");
  const Icon = ascending ? ArrowDownAZ : ArrowUpAZ;

  return (
    <IconTooltip tip={tip} placement="bottom">
      <button
        type="button"
        className={`${iconButtonClass} ${ascending ? "" : "bg-interactive-hover-strong"}`}
        aria-label={tip}
        disabled={busy}
        onClick={onToggle}
      >
        <Icon className={iconClass} />
      </button>
    </IconTooltip>
  );
}
