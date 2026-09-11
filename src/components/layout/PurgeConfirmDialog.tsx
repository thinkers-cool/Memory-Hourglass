import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { INPUT_CONTROL_FULL_CLASS } from "../../lib/formControlClass";

export function PurgeConfirmDialog({
  open,
  fileName,
  itemCount = 1,
  busy,
  onClose,
  onConfirm,
}: {
  open: boolean;
  fileName: string;
  itemCount?: number;
  busy: boolean;
  onClose: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation(["dialogs", "common"]);
  const confirmToken = t("dialogs:purge.confirmToken");
  const [value, setValue] = useState("");

  useEffect(() => {
    if (open) setValue("");
  }, [open]);

  if (!open) return null;

  const confirmed = value === confirmToken;
  const targetLabel =
    itemCount > 1
      ? t("dialogs:purge.targetItems", { count: itemCount })
      : fileName || t("dialogs:purge.targetFile");

  return (
    <dialog open className="modal modal-open">
      <div className="modal-box surface-card max-w-md">
        <h3 className="font-semibold text-lg mb-2">
          {itemCount > 1 ? t("dialogs:purge.titleItems") : t("dialogs:purge.titleFile")}
        </h3>
        <p className="text-sm text-content-muted mb-4">
          {t("dialogs:purge.message", { target: targetLabel })}
        </p>
        <fieldset className="fieldset">
          <legend className="fieldset-legend">{t("dialogs:purge.confirmPrompt")}</legend>
          <input
            type="text"
            className={`${INPUT_CONTROL_FULL_CLASS} w-full`}
            value={value}
            autoFocus
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && confirmed) onConfirm();
              if (e.key === "Escape") onClose();
            }}
          />
        </fieldset>
        <div className="modal-action">
          <button type="button" className="btn btn-ghost btn-interactive btn-sm" onClick={onClose}>
            {t("common:action.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-error btn-sm"
            disabled={busy || !confirmed}
            onClick={onConfirm}
          >
            {t("common:action.purge")}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button type="button" className="sr-only" onClick={onClose}>{t("common:action.close")}</button>
      </form>
    </dialog>
  );
}
