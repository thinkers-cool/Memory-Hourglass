import { describe, expect, it } from "vitest";
import { activityEventLabel, envelopeToMessage, renderMessage } from "./render";
import type { Message } from "./types";

describe("renderMessage", () => {
  it("returns empty string for null", () => {
    expect(renderMessage(null)).toBe("");
  });

  it("renders localized text from text_key", () => {
    const message: Message = {
      id: "1",
      kind: "success",
      text_key: "library:notification.rated",
      text_params: { count: 2 },
      duration_ms: 5000,
      created_at: 0,
    };
    expect(renderMessage(message)).toBe("Rated 2 items");
  });

  it("returns empty string when text is absent", () => {
    const message: Message = {
      id: "4",
      kind: "info",
      duration_ms: 0,
      created_at: 0,
    };
    expect(renderMessage(message)).toBe("");
  });

  it("trims plain text fallback", () => {
    const message: Message = {
      id: "3",
      kind: "info",
      text: "  disk full  ",
      duration_ms: 0,
      created_at: 0,
    };
    expect(renderMessage(message)).toBe("disk full");
  });

  it("falls back to plain text", () => {
    const message: Message = {
      id: "2",
      kind: "error",
      text: "disk full",
      duration_ms: 0,
      created_at: 0,
    };
    expect(renderMessage(message)).toBe("disk full");
  });
});

describe("envelopeToMessage", () => {
  it("maps backend envelopes into bus messages", () => {
    const message = envelopeToMessage({
      kind: "success",
      source: "user_action",
      text_key: "library:notification.deleted",
      text_params: { count: 1 },
      correlation_id: "corr-1",
      activity_id: 42,
      actions: [
        {
          label_key: "common:action.undo",
          action: "undo_activity",
          activity_id: 42,
        },
      ],
      duration_ms: 5000,
    });

    expect(message.kind).toBe("success");
    expect(message.text_key).toBe("library:notification.deleted");
    expect(message.activity_id).toBe(42);
    expect(message.actions?.[0]?.action).toBe("undo_activity");
    expect(message.duration_ms).toBe(5000);
  });

  it("uses default duration when omitted", () => {
    const message = envelopeToMessage({
      kind: "info",
      source: "system",
      text_key: "library:notification.saved",
      text_params: {},
      actions: [],
    });
    expect(message.duration_ms).toBe(5000);
  });
});

describe("activityEventLabel", () => {
  it("localizes known activity event types", () => {
    expect(activityEventLabel("asset.metadata_changed")).toBe("Rating changed");
  });

  it("returns the raw event type when no translation exists", () => {
    expect(activityEventLabel("asset.custom_event")).toBe("asset.custom_event");
  });
});
