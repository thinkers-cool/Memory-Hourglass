import type { FilterDef } from "../components/shared/FilterChipBar";
import i18n from "../i18n";

export function getMultiDisplayValue(filter: FilterDef, values: string[]): string {
  if (values.length === 0) return "";
  const labels = values.map((value) => filter.statusOptionLabels?.[value] ?? value);
  if (labels.length <= 2) return labels.join(", ");
  return `${labels[0]}, +${labels.length - 1}`;
}

export function getDisplayValue(
  filter: FilterDef,
  value: string,
  dateGte: string,
  dateLte: string,
): string {
  if (filter.type === "date") {
    if (dateGte && dateLte) {
      return i18n.t("common:date.range", { from: dateGte, to: dateLte });
    }
    if (dateGte) return i18n.t("common:date.fromDate", { date: dateGte });
    if (dateLte) return i18n.t("common:date.toDate", { date: dateLte });
    return "";
  }
  if (filter.type === "status") {
    return filter.statusOptionLabels?.[value] ?? value;
  }
  return value;
}
