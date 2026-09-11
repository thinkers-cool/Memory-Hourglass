import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { NavRail } from "./NavRail";

describe("NavRail", () => {
  it("switches tabs", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<NavRail active="library" onChange={onChange} onExit={vi.fn()} />);

    await user.click(screen.getByTitle("Collection"));
    expect(onChange).toHaveBeenCalledWith("collections");
  });

  it("closes workspace", async () => {
    const user = userEvent.setup();
    const onExit = vi.fn();
    render(<NavRail active="library" onChange={vi.fn()} onExit={onExit} />);

    await user.click(screen.getByTitle("Close workspace"));
    expect(onExit).toHaveBeenCalled();
  });

  it("shows theme control above close workspace", () => {
    render(<NavRail active="library" onChange={vi.fn()} onExit={vi.fn()} />);
    expect(screen.getByRole("button", { name: "Theme" })).toBeInTheDocument();
    expect(screen.getByTitle("Close workspace")).toBeInTheDocument();
  });
});
