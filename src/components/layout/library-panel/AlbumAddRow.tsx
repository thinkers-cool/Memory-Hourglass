import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import { DEFAULT_ALBUM_EMOJI } from "../../../lib/libraryIndicators";
import { EmojiPickerPopover } from "../../shared/EmojiPickerPopover";
import { IconTooltip } from "../../shared/Tooltip";
import { iconClass, inputClass } from "./panelStyles";

export function AlbumAddRow({
  placeholder,
  busy,
  onSubmit,
  onCancel,
}: {
  placeholder: string;
  busy: boolean;
  onSubmit: (name: string, emoji: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("common");
  const [value, setValue] = useState("");
  const [emoji, setEmoji] = useState(DEFAULT_ALBUM_EMOJI);

  const commit = () => {
    const trimmed = value.trim();
    if (!trimmed) return;
    onSubmit(trimmed, emoji);
    setValue("");
    setEmoji(DEFAULT_ALBUM_EMOJI);
  };

  return (
    <div className="mb-1 flex h-8 items-center gap-1.5 px-1">
      <EmojiPickerPopover value={emoji} disabled={busy} onChange={setEmoji} />
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
      <IconTooltip tip={t("action.save")} placement="bottom">
        <button
          type="button"
          className="btn btn-primary btn-sm btn-square h-8 min-h-0 w-8"
          aria-label={t("action.save")}
          disabled={busy || !value.trim()}
          onClick={commit}
        >
          <Check className={iconClass} />
        </button>
      </IconTooltip>
    </div>
  );
}
