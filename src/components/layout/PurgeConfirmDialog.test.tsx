import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { PurgeConfirmDialog } from "./PurgeConfirmDialog";

describe("PurgeConfirmDialog", () => {
  it("requires DELETE token before confirming", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(
      <PurgeConfirmDialog
        open
        fileName="photo.jpg"
        busy={false}
        onClose={vi.fn()}
        onConfirm={onConfirm}
      />,
    );

    const purge = screen.getByRole("button", { name: "Purge" });
    expect(purge).toBeDisabled();
    await user.type(screen.getByRole("textbox"), "DELETE");
    await user.click(purge);
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("supports multi-item purge copy and enter confirm", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(
      <PurgeConfirmDialog
        open
        fileName=""
        itemCount={3}
        busy={false}
        onClose={vi.fn()}
        onConfirm={onConfirm}
      />,
    );
    expect(screen.getByText(/3 items/i)).toBeInTheDocument();
    await user.type(screen.getByRole("textbox"), "DELETE{Enter}");
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("closes on escape and cancel", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <PurgeConfirmDialog
        open
        fileName="photo.jpg"
        busy={false}
        onClose={onClose}
        onConfirm={vi.fn()}
      />,
    );
    await user.type(screen.getByRole("textbox"), "{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("returns null when closed and disables while busy", () => {
    const { container, rerender } = render(
      <PurgeConfirmDialog
        open={false}
        fileName="photo.jpg"
        busy={false}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );
    expect(container.firstChild).toBeNull();

    rerender(
      <PurgeConfirmDialog
        open
        fileName="photo.jpg"
        busy
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );
    expect(screen.getByRole("button", { name: "Purge" })).toBeDisabled();
  });

  it("uses fallback target label without file name", () => {
    render(
      <PurgeConfirmDialog
        open
        fileName=""
        busy={false}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );
    expect(screen.getByText(/this file/i)).toBeInTheDocument();
  });
});
