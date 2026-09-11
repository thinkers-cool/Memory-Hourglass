import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import type { TagDto } from "../../../types";
import { DEFAULT_TAG_COLOR } from "../../../lib/libraryIndicators";
import { ColorPickerPopover } from "../../shared/ColorPickerPopover";
import { iconClass, inputClass } from "./panelStyles";

export function TagEditRow({
  tag,
  busy,
  onSave,
  onCancel,
}: {
  tag: TagDto;
  busy: boolean;
  onSave: (name: string, color: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("common");
  const [name, setName] = useState(tag.name);
  const [color, setColor] = useState(tag.color ?? DEFAULT_TAG_COLOR);

  const commit = () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    onSave(trimmed, color);
  };

  return (
    <div className="mb-1 flex h-8 items-center gap-1.5 px-1">
      <ColorPickerPopover value={color} disabled={busy} onChange={setColor} />
      <input
        type="text"
        className={inputClass}
        value={name}
        autoFocus
        disabled={busy}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") onCancel();
        }}
      />
      <button
        type="button"
        className="btn btn-primary btn-sm btn-square h-8 min-h-0 w-8"
        title={t("action.save")}
        disabled={busy || !name.trim()}
        onClick={commit}
      >
        <Check className={iconClass} />
      </button>
    </div>
  );
}
