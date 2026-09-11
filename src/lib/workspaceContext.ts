import i18n from "../i18n";
import type { RecentWorkspace } from "../types";

function countValue(value: number | undefined): number {
  return value ?? 0;
}

export function workspaceContextLabel(entry: RecentWorkspace): string {
  return [
    i18n.t("common:count.library", { count: countValue(entry.root_count) }),
    i18n.t("common:count.album", { count: countValue(entry.album_count) }),
    i18n.t("common:count.tag", { count: countValue(entry.tag_count) }),
  ].join(" · ");
}
