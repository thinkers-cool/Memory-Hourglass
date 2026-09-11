import { useMemo } from "react";
import { Search, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ghostBtnClass } from "../../lib/buttonClass";
import { INPUT_CONTROL_XS_CLASS } from "../../lib/formControlClass";
import { filterRawTags } from "../../lib/metadataSearch";
import type { RawTag } from "../../types";

export function MetadataFieldSearch({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  const { t } = useTranslation(["library", "common"]);
  const resolvedPlaceholder = placeholder ?? t("inspector.searchMetadata");

  return (
    <div className="relative min-w-0">
      <Search className="pointer-events-none absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-content-faint" />
      <input
        type="text"
        inputMode="search"
        enterKeyHint="search"
        className={`${INPUT_CONTROL_XS_CLASS} w-full py-0 pl-6 pr-6`}
        placeholder={resolvedPlaceholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") onChange("");
        }}
      />
      {value && (
        <button
          type="button"
          className={`${ghostBtnClass("btn-circle btn-xs absolute right-0.5 top-1/2 h-5 w-5 min-h-0 -translate-y-1/2 text-content-faint")}`}
          onClick={() => onChange("")}
          aria-label={t("common:filter.clearAll")}
        >
          <X className="h-2.5 w-2.5" />
        </button>
      )}
    </div>
  );
}

export function useFilteredRawTags(tags: RawTag[], query: string): RawTag[] {
  return useMemo(() => filterRawTags(tags, query), [tags, query]);
}
