import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { INPUT_CONTROL_FULL_CLASS } from "../../lib/formControlClass";
import { DEFAULT_ALBUM_EMOJI } from "../../lib/libraryIndicators";
import { EmojiPickerPopover } from "../shared/EmojiPickerPopover";

export function NamePromptDialog({
  open,
  title,
  label,
  submitLabel,
  busy,
  emojiPicker,
  onClose,
  onSubmit,
}: {
  open: boolean;
  title: string;
  label: string;
  submitLabel: string;
  busy: boolean;
  emojiPicker?: boolean;
  onClose: () => void;
  onSubmit: (value: string, emoji?: string) => void;
}) {
  const { t } = useTranslation("common");
  const [value, setValue] = useState("");
  const [emoji, setEmoji] = useState(DEFAULT_ALBUM_EMOJI);

  useEffect(() => {
    if (open) {
      setValue("");
      setEmoji(DEFAULT_ALBUM_EMOJI);
    }
  }, [open]);

  if (!open) return null;

  const submit = () => {
    const trimmed = value.trim();
    if (!trimmed) return;
    onSubmit(trimmed, emojiPicker ? emoji : undefined);
  };

  return (
    <dialog open className="modal modal-open">
      <div className="modal-box surface-card max-w-md">
        <h3 className="font-semibold text-lg mb-4">{title}</h3>
        {emojiPicker && (
          <div className="mb-3 flex items-center gap-2">
            <span className="text-xs opacity-60">{t("label.emoji")}</span>
            <EmojiPickerPopover
              value={emoji}
              disabled={busy}
              onChange={setEmoji}
            />
          </div>
        )}
        <fieldset className="fieldset">
          <legend className="fieldset-legend">{label}</legend>
          <input
            type="text"
            className={`${INPUT_CONTROL_FULL_CLASS} w-full`}
            value={value}
            autoFocus
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") submit();
              if (e.key === "Escape") onClose();
            }}
          />
        </fieldset>
        <div className="modal-action">
          <button
            type="button"
            className="btn btn-ghost btn-interactive btn-sm"
            onClick={onClose}
          >
            {t("action.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary btn-sm"
            disabled={busy || !value.trim()}
            onClick={submit}
          >
            {submitLabel}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button type="button" className="sr-only" onClick={onClose}>
          {t("action.close")}
        </button>
      </form>
    </dialog>
  );
}
