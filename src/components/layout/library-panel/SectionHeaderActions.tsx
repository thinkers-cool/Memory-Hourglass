import type { PanelOrderMode } from "../../../lib/panelOrder";
import { SectionOrderButton } from "./SectionOrderButton";

export function SectionHeaderActions({
  mode,
  busy,
  onToggle,
  children,
}: {
  mode: PanelOrderMode;
  busy: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className="flex shrink-0 items-center">
      <SectionOrderButton mode={mode} busy={busy} onToggle={onToggle} />
      {children}
    </div>
  );
}
