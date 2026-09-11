import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  SelectionPickerPopover,
  type PickerItem,
} from "./SelectionPickerPopover";

const items: PickerItem[] = [
  { key: "travel", label: "travel" },
  { key: "family", label: "family" },
];

describe("SelectionPickerPopover", () => {
  it("toggles item selection", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        open
        onOpenChange={vi.fn()}
        onToggle={onToggle}
      />,
    );
    await user.click(screen.getByText("travel"));
    expect(onToggle).toHaveBeenCalledWith("travel", true);
  });

  it("reflects selectedKeys from parent", () => {
    const { rerender } = render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        open
        selectedKeys={["travel"]}
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
      />,
    );
    expect(screen.getByText("travel").closest("button")).toHaveClass(
      "menu-picker-item-active",
    );

    rerender(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        open
        selectedKeys={[]}
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
      />,
    );
    expect(screen.getByText("travel").closest("button")).not.toHaveClass(
      "bg-interactive-selected",
    );
  });

  it("creates items from input", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue("new-tag");
    render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        open
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
        onCreate={onCreate}
      />,
    );
    const input = screen.getByPlaceholderText("New tag");
    await user.type(input, "new-tag{Enter}");
    expect(onCreate).toHaveBeenCalledWith("new-tag");
  });

  it("opens on hover and closes on escape", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        onOpenChange={onOpenChange}
        onToggle={vi.fn()}
      />,
    );
    await user.hover(screen.getByRole("button", { name: "Tag" }));
    expect(screen.getByText("travel")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Tag" }));
    expect(onOpenChange).toHaveBeenCalledWith(true);
    await user.type(screen.getByPlaceholderText("New tag"), "{Escape}");
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("deselects items in session mode and shows shortcut", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(
      <SelectionPickerPopover
        label="Tag"
        shortcut="T"
        items={items}
        placeholder="New tag"
        open
        onOpenChange={vi.fn()}
        onToggle={onToggle}
      />,
    );
    expect(screen.getByText("T")).toBeInTheDocument();
    await user.click(screen.getByText("travel"));
    await user.click(screen.getByText("travel"));
    expect(onToggle).toHaveBeenLastCalledWith("travel", false);
  });

  it("closes hover menu on mouse leave when not pinned", async () => {
    const user = userEvent.setup();
    render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
      />,
    );
    const trigger = screen.getByRole("button", { name: "Tag" });
    await user.hover(trigger);
    expect(screen.getByText("travel")).toBeInTheDocument();
    await user.unhover(trigger);
    expect(screen.queryByText("travel")).not.toBeInTheDocument();
  });

  it("clears session state when controlled open becomes false", () => {
    const { rerender } = render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        open
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
      />,
    );
    rerender(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        open={false}
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
      />,
    );
    expect(screen.queryByPlaceholderText("New tag")).not.toBeInTheDocument();
  });

  it("shows empty state and tracks session selection", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(
      <SelectionPickerPopover
        label="Tag"
        items={[]}
        placeholder="New tag"
        open
        onOpenChange={vi.fn()}
        onToggle={onToggle}
      />,
    );
    expect(screen.getByText("None yet")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Tag" }));
  });

  it("creates items without onCreate callback", async () => {
    const user = userEvent.setup();
    render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        open
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
      />,
    );
    await user.type(screen.getByPlaceholderText("New tag"), "local{Enter}");
    expect(screen.getByPlaceholderText("New tag")).toHaveValue("");
  });

  it("falls back to trimmed value when onCreate returns undefined", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue(undefined);
    render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        open
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
        onCreate={onCreate}
      />,
    );
    await user.type(screen.getByPlaceholderText("New tag"), "beach{Enter}");
    expect(onCreate).toHaveBeenCalledWith("beach");
  });

  it("ignores create while busy", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn();
    render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        busy
        open
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
        onCreate={onCreate}
      />,
    );
    await user.type(screen.getByPlaceholderText("New tag"), "beach{Enter}");
    expect(onCreate).not.toHaveBeenCalled();
  });

  it("ignores empty create input", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn();
    render(
      <SelectionPickerPopover
        label="Tag"
        items={items}
        placeholder="New tag"
        open
        onOpenChange={vi.fn()}
        onToggle={vi.fn()}
        onCreate={onCreate}
      />,
    );
    await user.type(screen.getByPlaceholderText("New tag"), "{Enter}");
    expect(onCreate).not.toHaveBeenCalled();
  });
});
