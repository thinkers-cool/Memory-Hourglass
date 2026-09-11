import type { Message, MessageInput } from "./types";

let currentToast: Message | null = null;
let currentBusy = false;
const listeners = new Set<() => void>();
const timers = new Map<string, number>();

function notify() {
  listeners.forEach((listener) => listener());
}

function createMessage(input: MessageInput): Message {
  return {
    id: crypto.randomUUID(),
    kind: input.kind,
    text_key: input.text_key,
    text_params: input.text_params,
    text: input.text,
    activity_id: input.activity_id,
    actions: input.actions,
    duration_ms: input.duration_ms ?? 5000,
    created_at: Date.now(),
  };
}

function scheduleDismiss(message: Message) {
  if (message.duration_ms <= 0) return;
  const existing = timers.get(message.id);
  if (existing !== undefined) {
    window.clearTimeout(existing);
  }
  const timer = window.setTimeout(() => {
    if (currentToast?.id === message.id) {
      currentToast = null;
      notify();
    }
    timers.delete(message.id);
  }, message.duration_ms);
  timers.set(message.id, timer);
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function getToast(): Message | null {
  return currentToast;
}

export function getBusy(): boolean {
  return currentBusy;
}

export function dispatchToast(input: MessageInput) {
  dispatchMessage(createMessage(input));
}

export function dispatchMessage(message: Message) {
  currentToast = message;
  notify();
  scheduleDismiss(message);
}

export function dismissToast(id?: string) {
  if (id && currentToast?.id !== id) return;
  if (!currentToast) return;
  const timer = timers.get(currentToast.id);
  if (timer !== undefined) window.clearTimeout(timer);
  currentToast = null;
  notify();
}

export function setBusyState(busy: boolean) {
  if (currentBusy === busy) return;
  currentBusy = busy;
  notify();
}
