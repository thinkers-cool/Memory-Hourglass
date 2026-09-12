import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { GridSizeControl } from "./GridSizeControl";

describe("GridSizeControl", () => {
  it("adjusts column count", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const onAdjust = vi.fn();
    render(
      <GridSizeControl value={5} onChange={onChange} onAdjust={onAdjust} />,
    );
    await user.click(screen.getByRole("button", { name: "View columns: 5" }));
    const buttons = screen.getAllByRole("button");
    await user.click(buttons[1]!);
    expect(onAdjust).toHaveBeenCalledWith(-1);
  });

  it("changes value from range input and plus control", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const onAdjust = vi.fn();
    render(
      <GridSizeControl value={5} onChange={onChange} onAdjust={onAdjust} />,
    );
    await user.click(screen.getByRole("button", { name: "View columns: 5" }));
    fireEvent.change(screen.getByRole("slider"), { target: { value: "8" } });
    expect(onChange).toHaveBeenCalledWith(8);
    await user.click(screen.getAllByRole("button")[2]!);
    expect(onAdjust).toHaveBeenCalledWith(1);
  });

  it("renders compact label and closes on outside click", async () => {
    const user = userEvent.setup();
    render(
      <GridSizeControl
        compact
        value={5}
        onChange={vi.fn()}
        onAdjust={vi.fn()}
      />,
    );
    expect(screen.queryByText("Grid")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "View columns: 5" }));
    expect(screen.getByRole("slider")).toBeInTheDocument();
    await user.click(document.body);
    expect(screen.queryByRole("slider")).not.toBeInTheDocument();
  });

  it("disables minus at minimum column count", async () => {
    const user = userEvent.setup();
    render(<GridSizeControl value={2} onChange={vi.fn()} onAdjust={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "View columns: 2" }));
    const buttons = screen.getAllByRole("button");
    expect(buttons[1]).toBeDisabled();
  });

  it("disables plus at maximum column count", async () => {
    const user = userEvent.setup();
    render(
      <GridSizeControl value={16} onChange={vi.fn()} onAdjust={vi.fn()} />,
    );
    await user.click(screen.getByRole("button", { name: "View columns: 16" }));
    const buttons = screen.getAllByRole("button");
    expect(buttons[2]).toBeDisabled();
  });
});
