import {
  listRowBadgeClass,
  listRowTitleClass,
} from "../../../lib/interactionClass";
import { Tooltip, TOOLTIP_DELAY_MS } from "../../shared/Tooltip";
import { panelRowActionButtonClass, panelRowButtonClass } from "./panelStyles";

function PanelRowTitle({
  title,
  active,
}: {
  title: string;
  active?: boolean;
}) {
  return (
    <span
      className={`block truncate text-xs leading-tight ${listRowTitleClass(active)}`}
    >
      {title}
    </span>
  );
}

function PanelRowBadge({
  active,
  badgeLabel,
  children,
}: {
  active?: boolean;
  badgeLabel?: string;
  children: React.ReactNode;
}) {
  return (
    <span
      aria-label={badgeLabel}
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
    <Tooltip
      tip={title}
      placement="left"
      delayMs={TOOLTIP_DELAY_MS}
      className="inline-flex"
    >
      <button
        type="button"
        className={panelRowActionButtonClass}
        aria-label={title}
        onClick={(event) => {
          event.stopPropagation();
          onClick();
        }}
      >
        {children}
      </button>
    </Tooltip>
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
  const pathTip =
    titleTooltip && titleTooltip !== title ? titleTooltip : undefined;

  const rowButton = (
    <button
      type="button"
      className={panelRowButtonClass(active, dimmed)}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
    >
      <PanelRowBadge active={active} badgeLabel={badgeTitle}>
        {badge}
      </PanelRowBadge>
      <span className="flex min-w-0 flex-1 flex-col gap-0.5 overflow-hidden pr-1">
        <PanelRowTitle title={title} active={active} />
        <span className="flex items-center gap-1.5 text-[10px] leading-none text-content-tertiary">
          {subtitle}
        </span>
      </span>
    </button>
  );

  return (
    <div className={`group relative mb-1 last:mb-0 ${indented ? "ml-3" : ""}`}>
      {pathTip ? (
        <Tooltip
          tip={pathTip}
          placement="bottom"
          delayMs={TOOLTIP_DELAY_MS}
          multiline
          className="block w-full min-w-0"
        >
          {rowButton}
        </Tooltip>
      ) : (
        rowButton
      )}
      {actions ? <PanelRowActions>{actions}</PanelRowActions> : null}
    </div>
  );
}
