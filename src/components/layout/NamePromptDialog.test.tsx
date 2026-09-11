import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { NamePromptDialog } from "./NamePromptDialog";

describe("NamePromptDialog", () => {
  it("submits trimmed name", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(
      <NamePromptDialog
        open
        title="New album"
        label="Name"
        submitLabel="Create"
        busy={false}
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />,
    );

    await user.type(screen.getByRole("textbox"), "  Summer  ");
    await user.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).toHaveBeenCalledWith("Summer", undefined);
  });

  it("does not submit empty names", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(
      <NamePromptDialog
        open
        title="New album"
        label="Name"
        submitLabel="Create"
        busy={false}
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />,
    );

    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("submits with emoji when picker is enabled", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(
      <NamePromptDialog
        open
        title="New album"
        label="Name"
        submitLabel="Create"
        busy={false}
        emojiPicker
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />,
    );
    await user.type(screen.getByRole("textbox"), "Summer");
    await user.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).toHaveBeenCalledWith("Summer", expect.any(String));
  });

  it("closes on escape and cancel", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <NamePromptDialog
        open
        title="New album"
        label="Name"
        submitLabel="Create"
        busy={false}
        onClose={onClose}
        onSubmit={vi.fn()}
      />,
    );
    await user.type(screen.getByRole("textbox"), "{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("submits on enter and hides when closed", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    const { container, rerender } = render(
      <NamePromptDialog
        open
        title="New album"
        label="Name"
        submitLabel="Create"
        busy={false}
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />,
    );
    await user.type(screen.getByRole("textbox"), "Trip{Enter}");
    expect(onSubmit).toHaveBeenCalledWith("Trip", undefined);

    rerender(
      <NamePromptDialog
        open={false}
        title="New album"
        label="Name"
        submitLabel="Create"
        busy={false}
        onClose={vi.fn()}
        onSubmit={vi.fn()}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("closes from backdrop button", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <NamePromptDialog
        open
        title="New album"
        label="Name"
        submitLabel="Create"
        busy={false}
        onClose={onClose}
        onSubmit={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("disables submit while busy", () => {
    render(
      <NamePromptDialog
        open
        title="New album"
        label="Name"
        submitLabel="Create"
        busy
        onClose={vi.fn()}
        onSubmit={vi.fn()}
      />,
    );
    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
  });

  it("ignores whitespace-only submit on enter", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(
      <NamePromptDialog
        open
        title="New album"
        label="Name"
        submitLabel="Create"
        busy={false}
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />,
    );
    await user.type(screen.getByRole("textbox"), "   {Enter}");
    expect(onSubmit).not.toHaveBeenCalled();
  });
});
