import { ghostBtnClass } from "../../../lib/buttonClass";
import { INPUT_CONTROL_CLASS } from "../../../lib/formControlClass";
import { listRowClass } from "../../../lib/interactionClass";

export const iconClass = "h-3.5 w-3.5";
export const iconButtonClass = ghostBtnClass(
  "btn-xs btn-square h-8 min-h-0 w-8 shrink-0",
);
export const panelRowActionButtonClass = ghostBtnClass(
  "btn-xs btn-square h-7 min-h-0 w-7 shrink-0 text-content-muted hover:text-base-content",
);
export function panelRowButtonClass(active?: boolean, dimmed?: boolean) {
  return `${ghostBtnClass("h-10 min-h-0 w-full min-w-0 items-center justify-start gap-2 rounded-lg p-1 text-left font-normal")} ${listRowClass(active)} ${
    dimmed ? "opacity-80" : ""
  }`;
}
export function rootDisplayName(path: string): string {
  const normalized = path.replace(/[/\\]+$/, "");
  const segments = normalized.split(/[/\\]/).filter(Boolean);
  return segments[segments.length - 1] ?? path;
}
export const inputClass = `${INPUT_CONTROL_CLASS} min-w-0 flex-1 px-2 text-xs`;
export const panelShellClass = "flex h-full flex-col border-r surface-panel";
