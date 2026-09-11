import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import * as api from "../../api/client";
import { ghostBtnClass } from "../../lib/buttonClass";
import { INPUT_CONTROL_FULL_CLASS } from "../../lib/formControlClass";
import type { SmbConnectParams } from "../../lib/libraryActions";
import { parseAppError, userFacingErrorMessage } from "../../lib/appError";
import { pickFolder } from "../../lib/pickFolder";
import { parseSmbHost } from "../../lib/smbAddress";
import { SmbFolderTree } from "../shared/SmbFolderTree";
import type { SmbShareEntry } from "../../types";
import { listRowClass } from "../../lib/interactionClass";

export type { SmbConnectParams };

type SmbDialogMode = "mounted" | "connect";
type ConnectStep = "credentials" | "shares" | "folders";

const modalBoxClass =
  "modal-box surface-card w-[min(100vw-2rem,36rem)] max-w-none p-6";

function ModeOption({
  active,
  title,
  onClick,
}: {
  active: boolean;
  title: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`${ghostBtnClass("h-auto min-h-0 w-full justify-start rounded-lg px-3 py-2.5 font-normal")} ${listRowClass(active)}`}
      onClick={onClick}
    >
      <div className={`text-sm font-medium ${active ? "text-primary" : ""}`}>
        {title}
      </div>
    </button>
  );
}

function ShareOption({
  active,
  share,
  onClick,
}: {
  active: boolean;
  share: SmbShareEntry;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`${ghostBtnClass("h-auto min-h-0 w-full justify-start rounded-lg px-3 py-2.5 font-normal")} ${listRowClass(active)}`}
      onClick={onClick}
    >
      <div className={`text-sm font-medium ${active ? "text-primary" : ""}`}>
        {share.name}
      </div>
      {share.comment && (
        <div className="mt-0.5 text-xs leading-relaxed text-content-muted">
          {share.comment}
        </div>
      )}
    </button>
  );
}

export function SmbConnectDialog({
  open,
  busy,
  onClose,
  onConnect,
  onAddMountedPath,
}: {
  open: boolean;
  busy: boolean;
  onClose: () => void;
  onConnect: (params: SmbConnectParams) => void;
  onAddMountedPath: (path: string) => void;
}) {
  const { t } = useTranslation(["dialogs", "common"]);
  const [mode, setMode] = useState<SmbDialogMode>("mounted");
  const [connectStep, setConnectStep] = useState<ConnectStep>("credentials");
  const [serverAddress, setServerAddress] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [pollSecs, setPollSecs] = useState(300);
  const [shares, setShares] = useState<SmbShareEntry[]>([]);
  const [selectedShare, setSelectedShare] = useState("");
  const [mountPath, setMountPath] = useState("");
  const [selectedFolderPath, setSelectedFolderPath] = useState("");
  const [listError, setListError] = useState("");
  const [listing, setListing] = useState(false);

  useEffect(() => {
    if (!open) {
      setMode("mounted");
      setConnectStep("credentials");
      setServerAddress("");
      setUsername("");
      setPassword("");
      setPollSecs(300);
      setShares([]);
      setSelectedShare("");
      setMountPath("");
      setSelectedFolderPath("");
      setListError("");
      setListing(false);
    }
  }, [open]);

  if (!open) return null;

  const host = parseSmbHost(serverAddress);
  const canSignIn =
    host !== null && username.trim().length > 0 && password.length > 0;
  const canContinueShare = selectedShare.length > 0;
  const canAddShare = mountPath.length > 0;

  const connectRequest = () => ({
    host: host as string,
    share: selectedShare,
    username: username.trim(),
    password,
  });

  const signIn = async () => {
    setListing(true);
    setListError("");
    try {
      const entries = await api.listSmbShares({
        host: host!,
        username: username.trim(),
        password,
      });
      if (entries.length === 0) {
        setListError(t("dialogs:smb.noDiskShares"));
        return;
      }
      setShares(entries);
      setSelectedShare(entries[0]?.name ?? "");
      setConnectStep("shares");
    } catch (error) {
      setListError(userFacingErrorMessage(parseAppError(error)));
    } finally {
      setListing(false);
    }
  };

  const continueToFolders = async () => {
    setListing(true);
    setListError("");
    try {
      const mountedPath = await api.mountSmbForBrowse(connectRequest());
      setMountPath(mountedPath);
      setSelectedFolderPath("");
      setConnectStep("folders");
    } catch (error) {
      setListError(userFacingErrorMessage(parseAppError(error)));
    } finally {
      setListing(false);
    }
  };

  const submitConnect = () => {
    onConnect({
      host: host!,
      share: selectedShare,
      username: username.trim(),
      password,
      pollSecs,
      folderPath: selectedFolderPath || undefined,
    });
  };

  const submitMounted = async () => {
    const path = await pickFolder({
      title: t("common:folderPicker.mountedSmb"),
    });
    if (!path) return;
    onAddMountedPath(path);
  };

  return (
    <dialog open className="modal modal-open">
      <div className={modalBoxClass}>
        <h3 className="mb-4 text-lg font-semibold">{t("dialogs:smb.title")}</h3>

        {connectStep === "credentials" && (
          <div className="mb-4 grid gap-2">
            <ModeOption
              active={mode === "mounted"}
              title={t("dialogs:smb.modeMountedTitle")}
              onClick={() => setMode("mounted")}
            />
            <ModeOption
              active={mode === "connect"}
              title={t("dialogs:smb.modeConnectTitle")}
              onClick={() => setMode("connect")}
            />
          </div>
        )}

        {mode === "mounted" ? null : connectStep === "credentials" ? (
          <div className="space-y-3">
            <fieldset className="fieldset">
              <legend className="fieldset-legend">
                {t("common:label.server")}
              </legend>
              <input
                type="text"
                className={`${INPUT_CONTROL_FULL_CLASS} w-full font-mono text-xs`}
                placeholder={t("dialogs:smb.serverPlaceholder")}
                value={serverAddress}
                onChange={(e) => setServerAddress(e.target.value)}
              />
            </fieldset>

            <fieldset className="fieldset">
              <legend className="fieldset-legend">
                {t("common:label.username")}
              </legend>
              <input
                type="text"
                className={`${INPUT_CONTROL_FULL_CLASS} w-full`}
                autoComplete="username"
                aria-label={t("common:label.username")}
                value={username}
                onChange={(e) => setUsername(e.target.value)}
              />
            </fieldset>

            <fieldset className="fieldset">
              <legend className="fieldset-legend">
                {t("common:label.password")}
              </legend>
              <input
                type="password"
                className={`${INPUT_CONTROL_FULL_CLASS} w-full`}
                autoComplete="current-password"
                aria-label={t("common:label.password")}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </fieldset>

            {listError && <p className="text-xs text-error">{listError}</p>}
          </div>
        ) : connectStep === "shares" ? (
          <div className="space-y-3">
            <p className="text-sm text-content-muted">
              {t("dialogs:smb.signedIn", { host })}
            </p>
            <div className="grid max-h-48 gap-2 overflow-y-auto">
              {shares.map((share) => (
                <ShareOption
                  key={share.name}
                  share={share}
                  active={selectedShare === share.name}
                  onClick={() => setSelectedShare(share.name)}
                />
              ))}
            </div>
            {listError && <p className="text-xs text-error">{listError}</p>}
          </div>
        ) : (
          <div className="space-y-3">
            <SmbFolderTree
              rootPath={mountPath}
              rootLabel={selectedShare}
              selectedRelativePath={selectedFolderPath}
              onSelect={setSelectedFolderPath}
            />
            <fieldset className="fieldset">
              <legend className="fieldset-legend">
                {t("dialogs:smb.rescanInterval")}
              </legend>
              <input
                type="number"
                min={30}
                className={`${INPUT_CONTROL_FULL_CLASS} w-full`}
                value={pollSecs}
                onChange={(e) => setPollSecs(Number(e.target.value) || 300)}
              />
            </fieldset>
          </div>
        )}

        <div className="modal-action mt-6">
          {mode === "connect" && connectStep === "folders" && (
            <button
              type="button"
              className="btn btn-ghost btn-interactive btn-sm"
              disabled={busy || listing}
              onClick={() => {
                setConnectStep("shares");
                setListError("");
              }}
            >
              {t("common:action.back")}
            </button>
          )}
          {mode === "connect" && connectStep === "shares" && (
            <button
              type="button"
              className="btn btn-ghost btn-interactive btn-sm"
              disabled={busy || listing}
              onClick={() => {
                setConnectStep("credentials");
                setListError("");
              }}
            >
              {t("common:action.back")}
            </button>
          )}
          <button
            type="button"
            className="btn btn-ghost btn-interactive btn-sm"
            onClick={onClose}
          >
            {t("common:action.cancel")}
          </button>
          {mode === "connect" ? (
            connectStep === "credentials" ? (
              <button
                type="button"
                className="btn btn-primary btn-sm"
                disabled={busy || listing || !canSignIn}
                onClick={() => void signIn()}
              >
                {listing ? t("common:busy.signingIn") : t("dialogs:smb.signIn")}
              </button>
            ) : connectStep === "shares" ? (
              <button
                type="button"
                className="btn btn-primary btn-sm"
                disabled={busy || listing || !canContinueShare}
                onClick={() => void continueToFolders()}
              >
                {listing
                  ? t("common:busy.connecting")
                  : t("common:action.continue")}
              </button>
            ) : (
              <button
                type="button"
                className="btn btn-primary btn-sm"
                disabled={busy || !canAddShare}
                onClick={submitConnect}
              >
                {t("dialogs:smb.addSource")}
              </button>
            )
          ) : (
            <button
              type="button"
              className="btn btn-primary btn-sm"
              disabled={busy}
              onClick={() => void submitMounted()}
            >
              {t("common:action.chooseFolder")}
            </button>
          )}
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
