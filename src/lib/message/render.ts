import i18n from "../../i18n";
import type { Message, MessageEnvelope } from "./types";

export function renderMessage(message: Message | null): string {
  if (!message) return "";
  if (message.text_key) {
    return i18n.t(message.text_key, message.text_params ?? {});
  }
  return message.text?.trim() ?? "";
}

export function envelopeToMessage(envelope: MessageEnvelope): Message {
  return {
    id: crypto.randomUUID(),
    kind: envelope.kind as Message["kind"],
    text_key: envelope.text_key,
    text_params: envelope.text_params,
    activity_id: envelope.activity_id,
    actions: envelope.actions,
    duration_ms: envelope.duration_ms ?? 5000,
    created_at: Date.now(),
  };
}

export function activityEventLabel(eventType: string): string {
  const key = `library:activity.${eventType.replace(/\./g, "_")}`;
  if (i18n.exists(key)) {
    return i18n.t(key);
  }
  return eventType;
}
