import { useTranslation } from "react-i18next";
import {
  INPUT_CONTROL_FULL_CLASS,
  INPUT_CONTROL_JOIN_CLASS,
  SELECT_CONTROL_CLASS,
} from "../../lib/formControlClass";
import { exportProgressLabel, isJobFinished } from "../../lib/jobProgress";
import { pickFolder } from "../../lib/pickFolder";
import type { ExportDialogState } from "../../lib/libraryActions";
import type { ExportOptions } from "../../types";

const FORMAT_OPTIONS = [
  { labelKey: "export.formatOriginal", value: "" },
  { labelKey: "export.formatJpeg", value: "jpeg" },
  { labelKey: "export.formatPng", value: "png" },
] as const;

export function ExportDialog({
  state,
  onClose,
  onUpdate,
  onStart,
}: {
  state: ExportDialogState;
  onClose: () => void;
  onUpdate: (patch: Partial<ExportDialogState>) => void;
  onStart: () => void;
}) {
  const { t } = useTranslation(["dialogs", "common"]);

  if (!state.open) return null;

  const exporting = state.jobId !== null;
  const exportDone =
    state.progress !== null && isJobFinished(state.progress.phase);
  const progressLabel = state.progress
    ? exportProgressLabel(state.progress)
    : "";

  const updateOptions = (patch: Partial<ExportOptions>) => {
    onUpdate({ options: { ...state.options, ...patch } });
  };

  const pickDestination = async () => {
    const path = await pickFolder();
    if (path) onUpdate({ destination: path });
  };

  return (
    <dialog open className="modal modal-open">
      <div className="modal-box surface-card max-w-md">
        <h3 className="font-semibold text-lg mb-4">{t("dialogs:export.title")}</h3>

        <p className="label text-sm mb-4">
          {t("dialogs:export.selected", { count: state.assetIds.length })}
        </p>

        <div className="space-y-3">
          <fieldset className="fieldset">
            <legend className="fieldset-legend">{t("dialogs:export.destination")}</legend>
            <div className="join w-full">
              <input
                type="text"
                className={INPUT_CONTROL_JOIN_CLASS}
                value={state.destination}
                readOnly
                placeholder={t("dialogs:export.chooseFolder")}
              />
              <button
                type="button"
                className="btn btn-outline btn-sm join-item"
                disabled={exporting}
                onClick={() => void pickDestination()}
              >
                {t("common:action.browse")}
              </button>
            </div>
          </fieldset>

          <fieldset className="fieldset">
            <legend className="fieldset-legend">{t("dialogs:export.layout")}</legend>
            <div className="join">
              <button
                type="button"
                className={`btn btn-sm join-item ${state.options.flat ? "btn-primary" : "btn-outline"}`}
                disabled={exporting}
                onClick={() => updateOptions({ flat: true })}
              >
                {t("dialogs:export.layoutFlat")}
              </button>
              <button
                type="button"
                className={`btn btn-sm join-item ${state.options.flat ? "btn-outline" : "btn-primary"}`}
                disabled={exporting}
                onClick={() => updateOptions({ flat: false })}
              >
                {t("dialogs:export.layoutStructured")}
              </button>
            </div>
            <p className="text-xs text-content-muted mt-1">
              {state.options.flat
                ? t("dialogs:export.layoutHintFlat")
                : t("dialogs:export.layoutHintStructured")}
            </p>
          </fieldset>

          <fieldset className="fieldset">
            <legend className="fieldset-legend">{t("dialogs:export.renameTemplate")}</legend>
            <input
              type="text"
              className={`${INPUT_CONTROL_FULL_CLASS} w-full`}
              value={state.options.rename_template ?? ""}
              disabled={exporting}
              onChange={(e) =>
                updateOptions({
                  rename_template: e.target.value.trim() ? e.target.value : undefined,
                })
              }
            />
            <p className="text-xs text-content-muted mt-1">
              {t("dialogs:export.renameTokens")}
            </p>
          </fieldset>

          <fieldset className="fieldset">
            <legend className="fieldset-legend">{t("dialogs:export.format")}</legend>
            <select
              className={SELECT_CONTROL_CLASS}
              value={state.options.format ?? ""}
              disabled={exporting}
              onChange={(e) =>
                updateOptions({ format: e.target.value || undefined })
              }
            >
              {FORMAT_OPTIONS.map((o) => (
                <option key={o.labelKey} value={o.value}>{t(`dialogs:${o.labelKey}`)}</option>
              ))}
            </select>
          </fieldset>

          {state.progress && (
            <fieldset className="fieldset">
              <legend className="fieldset-legend">{t("dialogs:export.progress")}</legend>
              <div className="flex justify-between gap-2 text-xs mb-1">
                <span className="truncate">{progressLabel}</span>
                <span className="shrink-0">{state.progress.done}/{state.progress.total}</span>
              </div>
              <progress
                className="progress progress-primary w-full"
                value={state.progress.done}
                max={state.progress.total || 1}
              />
            </fieldset>
          )}
        </div>

        <div className="modal-action">
          <button type="button" className="btn btn-ghost btn-interactive btn-sm" onClick={onClose}>
            {exportDone ? t("common:action.close") : t("common:action.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary btn-sm"
            disabled={exporting || !state.destination}
            onClick={onStart}
          >
            {exporting ? t("dialogs:export.exporting") : t("common:action.export")}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button type="button" className="sr-only" onClick={onClose}>{t("common:action.close")}</button>
      </form>
    </dialog>
  );
}
