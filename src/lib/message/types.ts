export type MessageKind = "success" | "error" | "info" | "warning";

export type MessageAction = {
  label_key: string;
  action: "undo_activity" | "dismiss";
  activity_id?: number;
};

export type MessageEnvelope = {
  kind: MessageKind;
  source: string;
  text_key: string;
  text_params: Record<string, string | number>;
  correlation_id?: string;
  activity_id?: number;
  actions: MessageAction[];
  duration_ms?: number;
};

export type Message = {
  id: string;
  kind: MessageKind;
  text_key?: string;
  text_params?: Record<string, string | number>;
  text?: string;
  activity_id?: number;
  actions?: MessageAction[];
  duration_ms: number;
  created_at: number;
};

export type MessageInput = {
  kind: MessageKind;
  text_key?: string;
  text_params?: Record<string, string | number>;
  text?: string;
  activity_id?: number;
  actions?: MessageAction[];
  duration_ms?: number;
};
