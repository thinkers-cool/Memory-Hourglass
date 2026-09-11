import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EmojiPickerPopover } from "./EmojiPickerPopover";

const loadEmojiCatalog = vi.hoisted(() =>
  vi.fn().mockResolvedValue(["📷", "🌅"]),
);

vi.mock("../../lib/emojiCatalog", () => ({
  loadEmojiCatalog,
}));

describe("EmojiPickerPopover", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads emojis when opened", async () => {
    const user = userEvent.setup();
    render(
      <EmojiPickerPopover value="📷" onChange={vi.fn()} title="Album emoji" />,
    );
    await user.click(screen.getByTitle("Album emoji"));
    expect((await screen.findAllByText("📷")).length).toBeGreaterThan(0);
    expect(loadEmojiCatalog).toHaveBeenCalledTimes(1);
  });

  it("calls onChange and closes when emoji is picked", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <EmojiPickerPopover value="" onChange={onChange} title="Album emoji" />,
    );
    await user.click(screen.getByTitle("Album emoji"));
    await user.click(await screen.findByText("🌅"));
    expect(onChange).toHaveBeenCalledWith("🌅");
    expect(screen.queryByText("🌅")).not.toBeInTheDocument();
  });

  it("does not open when disabled", async () => {
    const user = userEvent.setup();
    render(
      <EmojiPickerPopover
        value="📷"
        onChange={vi.fn()}
        title="Album emoji"
        disabled
      />,
    );
    await user.click(screen.getByTitle("Album emoji"));
    expect(loadEmojiCatalog).not.toHaveBeenCalled();
  });

  it("closes on outside click", async () => {
    const user = userEvent.setup();
    render(
      <>
        <EmojiPickerPopover value="📷" onChange={vi.fn()} title="Album emoji" />
        <button type="button">Outside</button>
      </>,
    );
    await user.click(screen.getByTitle("Album emoji"));
    expect(await screen.findByText("🌅")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(screen.queryByText("🌅")).not.toBeInTheDocument();
  });

  it("shows fallback emoji when value is empty", () => {
    render(
      <EmojiPickerPopover value="" onChange={vi.fn()} title="Album emoji" />,
    );
    expect(screen.getByTitle("Album emoji")).toHaveTextContent("😀");
  });
});
