import { Plus } from "lucide-react";
import { IconTooltip } from "../../shared/Tooltip";
import { iconButtonClass, iconClass } from "./panelStyles";

export function SectionAddButton({
  title,
  busy,
  active,
  onClick,
}: {
  title: string;
  busy: boolean;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <IconTooltip tip={title} placement="bottom">
      <button
        type="button"
        className={`${iconButtonClass} ${active ? "bg-interactive-hover-strong" : ""}`}
        aria-label={title}
        disabled={busy}
        onClick={onClick}
      >
        <Plus className={iconClass} />
      </button>
    </IconTooltip>
  );
}
