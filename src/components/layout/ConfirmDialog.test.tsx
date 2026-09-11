import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ConfirmDialog } from "./ConfirmDialog";

describe("ConfirmDialog", () => {
  it("returns null when closed", () => {
    const { container } = render(
      <ConfirmDialog
        open={false}
        title="Delete item"
        message="Are you sure?"
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("renders title and message when open", () => {
    render(
      <ConfirmDialog
        open
        title="Delete item"
        message="Are you sure?"
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );
    expect(screen.getByText("Delete item")).toBeInTheDocument();
    expect(screen.getByText("Are you sure?")).toBeInTheDocument();
  });

  it("calls confirm and cancel callbacks", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const onConfirm = vi.fn();
    render(
      <ConfirmDialog
        open
        title="Delete item"
        message="Are you sure?"
        confirmLabel="Delete"
        onClose={onClose}
        onConfirm={onConfirm}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Delete" }));
    expect(onConfirm).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("disables confirm while busy", () => {
    render(
      <ConfirmDialog
        open
        title="Delete item"
        message="Are you sure?"
        busy
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );
    expect(screen.getByRole("button", { name: "Confirm" })).toBeDisabled();
  });
});
