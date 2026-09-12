import { useRef, useState } from "react";
import { AlertTriangle, ChevronDown, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { MemHG } from "./MemHG";
import { StartStatusStrip } from "./layout/StartStatusStrip";
import { AppearanceControls } from "./shared/AppearanceControls";
import type { RecentWorkspace } from "../types";
import { ghostBtnClass } from "../lib/buttonClass";
import {
  MENU_ITEM_BUTTON_CLASS,
  MENU_POPOVER_CLASS,
} from "../lib/formControlClass";
import { listRowClass, listRowTitleClass } from "../lib/interactionClass";
import { workspaceContextLabel } from "../lib/workspaceContext";
import { usePopoverDismiss } from "../hooks/usePopoverDismiss";
import { rootDisplayName } from "./layout/library-panel/panelStyles";
import { IconTooltip, Tooltip, TOOLTIP_DELAY_MS } from "./shared/Tooltip";

function StartLeftPanel({
  busy,
  onOpen,
  onCreate,
}: {
  busy: boolean;
  onOpen: () => void;
  onCreate: (readOnly: boolean) => void;
}) {
  const { t } = useTranslation(["library", "common"]);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  usePopoverDismiss({
    open: menuOpen,
    onClose: () => setMenuOpen(false),
    containerRef: menuRef,
  });

  const createOptions = [
    { readOnly: false, label: t("library:start.createReadWrite") },
    { readOnly: true, label: t("library:start.createReadOnlyWorkspace") },
  ];

  return (
    <section className="flex flex-1 flex-col items-center justify-center gap-8 p-8 lg:p-12">
      <div className="flex w-full max-w-sm flex-col items-center gap-4 text-center">
        <MemHG size={72} title="Memory Hourglass" />
        <div className="space-y-1">
          <h1 className="text-3xl font-semibold tracking-tight">
            {t("common:app.name")}
          </h1>
          <p className="text-sm text-content-muted">
            {t("common:app.tagline")}
          </p>
        </div>
      </div>

      <div className="flex w-full max-w-sm flex-col gap-3">
        <div className="flex flex-wrap items-start justify-center gap-2">
          <button
            type="button"
            className="btn btn-primary min-w-36"
            disabled={busy}
            onClick={onOpen}
          >
            {t("library:start.openWorkspace")}
          </button>
          <div ref={menuRef} className="relative min-w-36">
            <button
              type="button"
              className="btn btn-outline w-full min-w-36 gap-1.5"
              disabled={busy}
              aria-expanded={menuOpen}
              aria-haspopup="menu"
              onClick={() => setMenuOpen((open) => !open)}
            >
              <span>{t("library:start.createWorkspace")}</span>
              <ChevronDown className="h-4 w-4 opacity-70" />
            </button>
            {menuOpen ? (
              <div className="surface-popover absolute left-0 top-full z-[60] mt-1 min-w-full p-1">
                <ul className={MENU_POPOVER_CLASS} role="menu">
                  {createOptions.map((option) => (
                    <li key={option.label} role="none">
                      <button
                        type="button"
                        role="menuitem"
                        className={MENU_ITEM_BUTTON_CLASS}
                        disabled={busy}
                        onClick={() => {
                          setMenuOpen(false);
                          onCreate(option.readOnly);
                        }}
                      >
                        {option.label}
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}
          </div>
        </div>
      </div>
    </section>
  );
}

function RegisteredWorkspaces({
  recent,
  busy,
  onOpenRecent,
  onRemoveRecent,
}: {
  recent: RecentWorkspace[];
  busy: boolean;
  onOpenRecent: (path: string) => void;
  onRemoveRecent: (path: string) => void;
}) {
  const { t } = useTranslation(["library", "common"]);

  return (
    <section className="flex min-h-0 flex-1 flex-col justify-start p-8 lg:p-12">
      <h2 className="mb-3 text-xs font-medium uppercase tracking-wider text-content-faint">
        {t("start.workspacesHeading")}
      </h2>

      <ul className="flex flex-col gap-1.5 overflow-y-auto">
        {recent.map((entry) => (
          <li key={entry.path} className="group relative">
            <button
              type="button"
              className={`${ghostBtnClass("flex h-auto min-h-10 w-full flex-col items-start justify-center gap-1 rounded-lg px-2.5 py-2 text-left font-normal")} ${listRowClass()} pr-10`}
              disabled={busy}
              onClick={() => onOpenRecent(entry.path)}
            >
              <Tooltip
                tip={entry.path}
                placement="bottom"
                delayMs={TOOLTIP_DELAY_MS}
                multiline
                className="flex w-full min-w-0 items-center gap-2 text-xs leading-snug"
              >
                <span
                  className={`min-w-0 truncate ${listRowTitleClass(entry.valid)}`}
                >
                  {rootDisplayName(entry.path)}
                </span>
                {entry.read_only ? (
                  <span className="badge badge-outline badge-xs shrink-0 font-normal normal-case">
                    {t("start.readOnlyBadge")}
                  </span>
                ) : null}
              </Tooltip>
              {entry.valid ? (
                <span className="block w-full truncate text-[10px] leading-snug text-content-tertiary">
                  {workspaceContextLabel(entry)}
                </span>
              ) : (
                <span className="flex w-full items-center gap-1 text-[10px] leading-snug text-warning">
                  <AlertTriangle className="h-3 w-3 shrink-0" />
                  <span>{t("start.pathNotFound")}</span>
                </span>
              )}
            </button>
            <IconTooltip
              tip={t("start.removeWorkspace")}
              placement="left"
              className="absolute right-1 top-1/2 -translate-y-1/2"
            >
              <button
                type="button"
                className="btn btn-ghost btn-interactive btn-square btn-xs h-7 min-h-0 w-7 shrink-0 opacity-60 group-hover:opacity-100"
                aria-label={t("start.removeWorkspace")}
                disabled={busy}
                onClick={() => onRemoveRecent(entry.path)}
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </IconTooltip>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function StartPage({
  recent,
  busy,
  busyMessage,
  notification,
  onDismissAlert,
  onCreate,
  onOpen,
  onOpenRecent,
  onRemoveRecent,
}: {
  recent: RecentWorkspace[];
  busy: boolean;
  busyMessage: string;
  notification: import("../types").Notification | null;
  onDismissAlert?: () => void;
  onCreate: (readOnly: boolean) => void;
  onOpen: () => void;
  onOpenRecent: (path: string) => void;
  onRemoveRecent: (path: string) => void;
}) {
  const hasRegisteredWorkspaces = recent.length > 0;

  return (
    <div className="relative flex min-h-screen app-canvas">
      <header className="absolute right-4 top-4 z-50">
        <AppearanceControls
          menuPlacement="float-bottom-end"
          layout="horizontal"
        />
      </header>

      <StartLeftPanel busy={busy} onOpen={onOpen} onCreate={onCreate} />

      {hasRegisteredWorkspaces ? (
        <RegisteredWorkspaces
          recent={recent}
          busy={busy}
          onOpenRecent={onOpenRecent}
          onRemoveRecent={onRemoveRecent}
        />
      ) : null}

      <StartStatusStrip
        busy={busy}
        busyMessage={busyMessage}
        notification={notification}
        onDismissAlert={onDismissAlert}
      />
    </div>
  );
}
