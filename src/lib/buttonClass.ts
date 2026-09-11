export const GHOST_BTN_INTERACTIVE = "btn btn-ghost btn-interactive";

export function ghostBtnClass(extra = ""): string {
  return extra ? `${GHOST_BTN_INTERACTIVE} ${extra}` : GHOST_BTN_INTERACTIVE;
}
