import { useTranslation } from "react-i18next";

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel,
  busy,
  onClose,
  onConfirm,
}: {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  busy?: boolean;
  onClose: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation(["dialogs", "common"]);

  if (!open) return null;

  return (
    <dialog open className="modal modal-open">
      <div className="modal-box surface-card max-w-md">
        <h3 className="font-semibold text-lg mb-2">{title}</h3>
        <p className="text-sm text-content-muted mb-4">{message}</p>
        <div className="modal-action">
          <button
            type="button"
            className="btn btn-ghost btn-interactive btn-sm"
            onClick={onClose}
          >
            {t("common:action.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary btn-sm"
            disabled={busy}
            onClick={onConfirm}
          >
            {confirmLabel ?? t("dialogs:confirm.defaultConfirmLabel")}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button type="button" className="sr-only" onClick={onClose}>
          {t("common:action.close")}
        </button>
      </form>
    </dialog>
  );
}
