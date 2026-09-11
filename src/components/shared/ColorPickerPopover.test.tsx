import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ColorPickerPopover } from "./ColorPickerPopover";

vi.mock("react-colorful", () => ({
  HexColorPicker: ({
    onChange,
  }: {
    color: string;
    onChange: (color: string) => void;
  }) => (
    <button
      type="button"
      className="mock-color-picker"
      onClick={() => onChange("#00ff00")}
    >
      Pick
    </button>
  ),
}));

describe("ColorPickerPopover", () => {
  it("opens color picker popover", async () => {
    const user = userEvent.setup();
    render(
      <ColorPickerPopover
        value="#ff0000"
        onChange={vi.fn()}
        title="Tag color"
      />,
    );
    await user.click(screen.getByTitle("Tag color"));
    expect(document.querySelector(".mock-color-picker")).toBeInTheDocument();
  });

  it("calls onChange when a color is picked", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <ColorPickerPopover
        value="#ff0000"
        onChange={onChange}
        title="Tag color"
      />,
    );
    await user.click(screen.getByTitle("Tag color"));
    await user.click(screen.getByRole("button", { name: "Pick" }));
    expect(onChange).toHaveBeenCalledWith("#00ff00");
  });

  it("does not open when disabled", async () => {
    const user = userEvent.setup();
    render(
      <ColorPickerPopover
        value="#ff0000"
        onChange={vi.fn()}
        title="Tag color"
        disabled
      />,
    );
    await user.click(screen.getByTitle("Tag color"));
    expect(
      document.querySelector(".mock-color-picker"),
    ).not.toBeInTheDocument();
  });

  it("closes on outside click", async () => {
    const user = userEvent.setup();
    render(
      <>
        <ColorPickerPopover
          value="#ff0000"
          onChange={vi.fn()}
          title="Tag color"
        />
        <button type="button">Outside</button>
      </>,
    );
    await user.click(screen.getByTitle("Tag color"));
    expect(document.querySelector(".mock-color-picker")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(
      document.querySelector(".mock-color-picker"),
    ).not.toBeInTheDocument();
  });

  it("toggles closed on second click", async () => {
    const user = userEvent.setup();
    render(
      <ColorPickerPopover
        value="#ff0000"
        onChange={vi.fn()}
        title="Tag color"
      />,
    );
    await user.click(screen.getByTitle("Tag color"));
    expect(document.querySelector(".mock-color-picker")).toBeInTheDocument();
    await user.click(screen.getByTitle("Tag color"));
    expect(
      document.querySelector(".mock-color-picker"),
    ).not.toBeInTheDocument();
  });
});
