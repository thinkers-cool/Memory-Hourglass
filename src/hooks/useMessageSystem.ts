import { useCallback, useEffect, useState } from "react";
import * as api from "../api/client";
import { errorNotification } from "../lib/appError";
import {
  dismissToast,
  dispatchMessage,
  dispatchToast,
  getBusy,
  getToast,
  setBusyState,
  subscribe,
} from "../lib/message/bus";
import { envelopeToMessage, renderMessage } from "../lib/message/render";
import type { Message, MessageInput } from "../lib/message/types";
import type { Notification } from "../types";

function messageAsNotification(message: Message | null): Notification | null {
  if (!message) return null;
  return {
    kind: message.kind === "warning" ? "info" : message.kind,
    text: renderMessage(message),
    activity_id: message.activity_id,
    actions: message.actions,
  };
}

export function useMessageSystem() {
  const [toast, setToast] = useState<Message | null>(() => getToast());
  const [busy, setBusy] = useState(() => getBusy());

  useEffect(() => {
    return subscribe(() => {
      setToast(getToast());
      setBusy(getBusy());
    });
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void api
      .onMessageNotify((envelope) => {
        dispatchMessage(envelopeToMessage(envelope));
      })
      .then((fn) => {
        unlisten = fn;
      });
    return () => unlisten?.();
  }, []);

  const dispatch = useCallback((input: MessageInput) => {
    dispatchToast(input);
  }, []);

  const reportError = useCallback((error: unknown) => {
    const notification = errorNotification(error);
    dispatchToast({
      kind: "error",
      text: notification.text,
      duration_ms: 0,
    });
  }, []);

  const withBusy = useCallback(
    async (fn: () => Promise<void>) => {
      setBusyState(true);
      try {
        await fn();
      } catch (error) {
        reportError(error);
        throw error;
      } finally {
        setBusyState(false);
      }
    },
    [reportError],
  );

  const undoActivity = useCallback(
    async (activityId: number) => {
      await withBusy(async () => {
        await api.undoActivity(activityId);
        dispatchToast({
          kind: "success",
          text_key: "library:notification.undone",
          duration_ms: 4000,
        });
      });
    },
    [withBusy],
  );

  const notification = messageAsNotification(toast);

  const setNotification = useCallback((value: Notification | null) => {
    if (!value) {
      dismissToast();
      return;
    }
    dispatchToast({
      kind: value.kind,
      text: value.text,
      activity_id: value.activity_id,
      actions: value.actions,
    });
  }, []);

  return {
    toast,
    notification,
    dispatch,
    reportError,
    busy,
    withBusy,
    dismissToast,
    undoActivity,
    setNotification,
  };
}
