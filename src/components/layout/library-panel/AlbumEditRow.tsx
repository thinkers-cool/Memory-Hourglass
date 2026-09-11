import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import type { Album } from "../../../types";
import { DEFAULT_ALBUM_EMOJI } from "../../../lib/libraryIndicators";
import { EmojiPickerPopover } from "../../shared/EmojiPickerPopover";
import { iconClass, inputClass } from "./panelStyles";

export function AlbumEditRow({
  album,
  busy,
  onSave,
  onCancel,
}: {
  album: Album;
  busy: boolean;
  onSave: (name: string, emoji: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation("common");
  const [name, setName] = useState(album.name);
  const [emoji, setEmoji] = useState(album.emoji ?? DEFAULT_ALBUM_EMOJI);

  const commit = () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    onSave(trimmed, emoji);
  };

  return (
    <div className="mb-1 flex h-8 items-center gap-1.5 px-1">
      <EmojiPickerPopover value={emoji} disabled={busy} onChange={setEmoji} />
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
