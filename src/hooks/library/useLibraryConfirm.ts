import { useCallback, useState } from "react";

export type ConfirmDialogState = {
  open: boolean;
  title: string;
  message: string;
  onConfirm: () => void | Promise<void>;
};

const closedConfirmDialog: ConfirmDialogState = {
  open: false,
  title: "",
  message: "",
  onConfirm: () => {},
};

export function useLibraryConfirm() {
  const [confirmDialog, setConfirmDialog] =
    useState<ConfirmDialogState>(closedConfirmDialog);

  const requestConfirm = useCallback(
    (title: string, message: string, onConfirm: () => void | Promise<void>) => {
      setConfirmDialog({ open: true, title, message, onConfirm });
    },
    [],
  );

  const closeConfirmDialog = useCallback(() => {
    setConfirmDialog((prev) => ({ ...prev, open: false }));
  }, []);

  return { confirmDialog, requestConfirm, closeConfirmDialog };
}
