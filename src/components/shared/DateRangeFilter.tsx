import { useTranslation } from "react-i18next";
import { INPUT_CONTROL_CLASS } from "../../lib/formControlClass";

interface DateRangeFilterProps {
  dateGte: string;
  dateLte: string;
  onChange: (dateGte: string, dateLte: string) => void;
  size?: "sm" | "xs";
}

export function DateRangeFilter({
  dateGte,
  dateLte,
  onChange,
  size = "sm",
}: DateRangeFilterProps) {
  const { t } = useTranslation("common");
  const inputCls =
    size === "xs"
      ? `${INPUT_CONTROL_CLASS} input-xs text-xs min-w-0 w-full`
      : `${INPUT_CONTROL_CLASS} text-sm min-w-0 w-full`;

  const hasValue = dateGte !== "" || dateLte !== "";

  return (
    <div className="inline-flex items-center gap-1.5 rounded-lg border border-control-border bg-control/65 px-2 py-1">
      <input
        className={inputCls}
        title={t("date.from")}
        type="date"
        value={dateGte}
        onChange={(e) => onChange(e.target.value, dateLte)}
      />
      <span className="text-content-quaternary text-xs">–</span>
      <input
        className={inputCls}
        title={t("date.to")}
        type="date"
        value={dateLte}
        onChange={(e) => onChange(dateGte, e.target.value)}
      />
      {hasValue && (
        <button
          className="btn btn-ghost btn-interactive btn-circle btn-xs h-5 w-5 min-h-0"
          title={t("date.clearDates")}
          type="button"
          onClick={() => onChange("", "")}
        >
          ×
        </button>
      )}
    </div>
  );
}
