import { Bookmark, Library, LogOut } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { MemHG } from "../MemHG";
import { AppearanceControls } from "../shared/AppearanceControls";

const TABS: { id: "library" | "collections"; labelKey: string; icon: LucideIcon }[] = [
  { id: "library", labelKey: "nav.library", icon: Library },
  { id: "collections", labelKey: "nav.collection", icon: Bookmark },
];

export type LeftTab = (typeof TABS)[number]["id"];

export function NavRail({
  active,
  onChange,
  onExit,
}: {
  active: LeftTab;
  onChange: (tab: LeftTab) => void;
  onExit: () => void;
}) {
  const { t } = useTranslation("library");

  return (
    <nav className="surface-panel flex h-full w-14 shrink-0 flex-col items-center border-r py-3">
      <div className="mb-4 px-1">
        <MemHG size={34} title="Memory Hourglass" />
      </div>
      <ul className="menu menu-sm menu-vertical w-full flex-1 items-center gap-1.5 p-0">
        {TABS.map((tab) => {
          const Icon = tab.icon;
          const label = t(tab.labelKey);
          return (
            <li key={tab.id}>
              <button
                type="button"
                title={label}
                className={`btn btn-ghost btn-interactive btn-square h-11 min-h-0 w-11 ${
                  active === tab.id ? "menu-active bg-interactive-selected-strong" : ""
                }`}
                onClick={() => onChange(tab.id)}
              >
                <Icon className="h-5 w-5" />
              </button>
            </li>
          );
        })}
      </ul>
      <div className="flex flex-col items-center gap-1.5">
        <AppearanceControls menuPlacement="float-top-end" size="nav" />
        <button
          type="button"
          title={t("nav.closeWorkspace")}
          className="btn btn-ghost btn-interactive btn-square h-11 min-h-0 w-11 shrink-0"
          onClick={onExit}
        >
          <LogOut className="h-5 w-5" />
        </button>
      </div>
    </nav>
  );
}

export function PanelSection({
  title,
  action,
  children,
}: {
  title: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <fieldset className="fieldset mb-4 p-0">
      <div className="mb-2 flex min-h-8 items-center justify-between gap-2 px-1">
        <legend className="fieldset-legend min-w-0 flex-1 truncate text-xs font-medium uppercase tracking-wider text-content-faint p-0">
          {title}
        </legend>
        {action}
      </div>
      {children}
    </fieldset>
  );
}
