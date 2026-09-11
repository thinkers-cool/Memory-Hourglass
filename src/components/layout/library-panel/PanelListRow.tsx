import {
  listRowBadgeClass,
  listRowTitleClass,
} from "../../../lib/interactionClass";
import { panelRowActionButtonClass, panelRowButtonClass } from "./panelStyles";

function PanelRowBadge({
  active,
  title,
  children,
}: {
  active?: boolean;
  title?: string;
  children: React.ReactNode;
}) {
  return (
    <span
      title={title}
      className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-lg ${listRowBadgeClass(active)}`}
    >
      {children}
    </span>
  );
}

export function PanelRowActionButton({
  title,
  onClick,
  children,
}: {
  title: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      className={panelRowActionButtonClass}
      title={title}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
    >
      {children}
    </button>
  );
}

function PanelRowActions({ children }: { children: React.ReactNode }) {
  return (
    <div className="surface-chip pointer-events-none absolute right-1 top-1/2 flex -translate-y-1/2 items-center gap-0.5 px-0.5 opacity-0 transition-opacity group-hover:pointer-events-auto group-hover:opacity-100">
      {children}
    </div>
  );
}

export function PanelListRow({
  active,
  dimmed,
  indented,
  badge,
  badgeTitle,
  title,
  titleTooltip,
  subtitle,
  onClick,
  onDoubleClick,
  actions,
}: {
  active?: boolean;
  dimmed?: boolean;
  indented?: boolean;
  badge: React.ReactNode;
  badgeTitle?: string;
  title: string;
  titleTooltip?: string;
  subtitle: React.ReactNode;
  onClick: () => void;
  onDoubleClick?: () => void;
  actions?: React.ReactNode;
}) {
  return (
    <div className={`group relative mb-1 last:mb-0 ${indented ? "ml-3" : ""}`}>
      <button
        type="button"
        className={panelRowButtonClass(active, dimmed)}
        onClick={onClick}
        onDoubleClick={onDoubleClick}
      >
        <PanelRowBadge active={active} title={badgeTitle}>
          {badge}
        </PanelRowBadge>
        <span className="flex min-w-0 flex-1 flex-col gap-0.5 overflow-hidden pr-1">
          <span
            className={`truncate text-xs leading-tight ${listRowTitleClass(active)}`}
            title={titleTooltip ?? title}
          >
            {title}
          </span>
          <span className="flex items-center gap-1.5 text-[10px] leading-none text-content-tertiary">
            {subtitle}
          </span>
        </span>
      </button>
      {actions ? <PanelRowActions>{actions}</PanelRowActions> : null}
    </div>
  );
}
