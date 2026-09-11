import { Plus } from "lucide-react";
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
    <button
      type="button"
      className={`${iconButtonClass} ${active ? "bg-interactive-hover-strong" : ""}`}
      title={title}
      disabled={busy}
      onClick={onClick}
    >
      <Plus className={iconClass} />
    </button>
  );
}
