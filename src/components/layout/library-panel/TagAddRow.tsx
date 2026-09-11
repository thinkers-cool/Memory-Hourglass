import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import { DEFAULT_TAG_COLOR } from "../../../lib/libraryIndicators";
import { ColorPickerPopover } from "../../shared/ColorPickerPopover";
import { iconClass, inputClass } from "./panelStyles";

export function TagAddRow({
  placeholder,
  busy,
  indented = false,
  onSubmit,
  onCancel,
}: {
  placeholder: string;
  busy: boolean;
  indented?: boolean;
  onSubmit: (name: string, color: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("common");
  const [value, setValue] = useState("");
  const [color, setColor] = useState(DEFAULT_TAG_COLOR);

  const commit = () => {
    const trimmed = value.trim();
    if (!trimmed) return;
    onSubmit(trimmed, color);
    setValue("");
    setColor(DEFAULT_TAG_COLOR);
  };

  return (
    <div
      className={`mb-1 flex h-8 items-center gap-1.5 px-1 ${indented ? "ml-3" : ""}`}
    >
      <ColorPickerPopover value={color} disabled={busy} onChange={setColor} />
      <input
        type="text"
        className={inputClass}
        placeholder={placeholder}
        value={value}
        autoFocus
        disabled={busy}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") onCancel();
        }}
      />
      <button
        type="button"
        className="btn btn-primary btn-sm btn-square h-8 min-h-0 w-8"
        title={t("action.save")}
        disabled={busy || !value.trim()}
        onClick={commit}
      >
        <Check className={iconClass} />
      </button>
    </div>
  );
}
