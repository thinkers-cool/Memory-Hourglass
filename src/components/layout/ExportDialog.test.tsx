import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ExportDialog } from "./ExportDialog";
import type { ExportDialogState } from "../../lib/libraryActions";

const pickFolder = vi.hoisted(() => vi.fn());

vi.mock("../../lib/pickFolder", () => ({
  pickFolder,
}));

const baseState: ExportDialogState = {
  open: true,
  assetIds: [1, 2],
  destination: "",
  options: { flat: true, rename_template: undefined, format: undefined },
  jobId: null,
  progress: null,
};

describe("ExportDialog", () => {
  it("returns null when closed", () => {
    const { container } = render(
      <ExportDialog
        state={{ ...baseState, open: false }}
        onClose={vi.fn()}
        onUpdate={vi.fn()}
        onStart={vi.fn()}
      />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("updates export options", async () => {
    const user = userEvent.setup();
    const onUpdate = vi.fn();
    render(
      <ExportDialog
        state={baseState}
        onClose={vi.fn()}
        onUpdate={onUpdate}
        onStart={vi.fn()}
      />,
    );

    await user.selectOptions(screen.getByRole("combobox"), "jpeg");
    expect(onUpdate).toHaveBeenCalledWith({
      options: { flat: true, rename_template: undefined, format: "jpeg" },
    });
  });

  it("picks destination folder via browse", async () => {
    const user = userEvent.setup();
    pickFolder.mockResolvedValue("/tmp/export");
    const onUpdate = vi.fn();
    render(
      <ExportDialog
        state={baseState}
        onClose={vi.fn()}
        onUpdate={onUpdate}
        onStart={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Browse" }));
    await vi.waitFor(() => {
      expect(onUpdate).toHaveBeenCalledWith({ destination: "/tmp/export" });
    });
  });

  it("shows structured layout hint when flat is false", () => {
    render(
      <ExportDialog
        state={{
          ...baseState,
          options: { flat: false, rename_template: undefined, format: undefined },
        }}
        onClose={vi.fn()}
        onUpdate={vi.fn()}
        onStart={vi.fn()}
      />,
    );
    expect(screen.getByText(/Preserves subfolders/i)).toBeInTheDocument();
  });

  it("ignores browse when no folder is selected", async () => {
    const user = userEvent.setup();
    pickFolder.mockResolvedValue(null);
    const onUpdate = vi.fn();
    render(
      <ExportDialog
        state={baseState}
        onClose={vi.fn()}
        onUpdate={onUpdate}
        onStart={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Browse" }));
    await vi.waitFor(() => {
      expect(pickFolder).toHaveBeenCalled();
    });
    expect(onUpdate).not.toHaveBeenCalled();
  });

  it("switches layout between flat and structured", async () => {
    const user = userEvent.setup();
    const onUpdate = vi.fn();
    render(
      <ExportDialog
        state={baseState}
        onClose={vi.fn()}
        onUpdate={onUpdate}
        onStart={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Structured" }));
    expect(onUpdate).toHaveBeenCalledWith({
      options: { flat: false, rename_template: undefined, format: undefined },
    });
    await user.click(screen.getByRole("button", { name: "Flat" }));
    expect(onUpdate).toHaveBeenLastCalledWith({
      options: { flat: true, rename_template: undefined, format: undefined },
    });
  });

  it("updates rename template", async () => {
    const user = userEvent.setup();
    const onUpdate = vi.fn();
    render(
      <ExportDialog
        state={baseState}
        onClose={vi.fn()}
        onUpdate={onUpdate}
        onStart={vi.fn()}
      />,
    );
    await user.type(screen.getAllByRole("textbox")[1], "photo_name");
    expect(onUpdate).toHaveBeenCalled();
  });

  it("starts export when destination is set", async () => {
    const user = userEvent.setup();
    const onStart = vi.fn();
    render(
      <ExportDialog
        state={{ ...baseState, destination: "/tmp/out" }}
        onClose={vi.fn()}
        onUpdate={vi.fn()}
        onStart={onStart}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Export" }));
    expect(onStart).toHaveBeenCalledTimes(1);
  });

  it("calls onClose from cancel", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <ExportDialog
        state={baseState}
        onClose={onClose}
        onUpdate={vi.fn()}
        onStart={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("shows progress and close label when export completes", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <ExportDialog
        state={{
          ...baseState,
          destination: "/tmp/out",
          jobId: "job-1",
          progress: {
            done: 2,
            total: 2,
            phase: "completed",
            message: "Export complete",
          },
        }}
        onClose={onClose}
        onUpdate={vi.fn()}
        onStart={vi.fn()}
      />,
    );
    expect(screen.getByText("Export complete")).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("value", "2");
    await user.click(
      screen.getByRole("dialog").querySelector(".modal-action button.btn-ghost") as Element,
    );
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("disables controls while exporting", () => {
    render(
      <ExportDialog
        state={{
          ...baseState,
          destination: "/tmp/out",
          jobId: "job-1",
          progress: {
            done: 1,
            total: 2,
            phase: "running",
            message: "Exporting",
            file_name: "photo.jpg",
          },
        }}
        onClose={vi.fn()}
        onUpdate={vi.fn()}
        onStart={vi.fn()}
      />,
    );
    expect(screen.getByRole("button", { name: "Browse" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Exporting…" })).toBeDisabled();
    expect(screen.getByText(/Exporting: photo\.jpg/)).toBeInTheDocument();
  });

  it("clears rename template and format when emptied", async () => {
    const user = userEvent.setup();
    const onUpdate = vi.fn();
    render(
      <ExportDialog
        state={{
          ...baseState,
          options: { flat: true, rename_template: "name", format: "jpeg" },
        }}
        onClose={vi.fn()}
        onUpdate={onUpdate}
        onStart={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByDisplayValue("name"), { target: { value: "   " } });
    expect(onUpdate).toHaveBeenCalledWith({
      options: { flat: true, rename_template: undefined, format: "jpeg" },
    });
    await user.selectOptions(screen.getByRole("combobox"), "");
    expect(onUpdate).toHaveBeenCalledWith({
      options: { flat: true, rename_template: "name", format: undefined },
    });
  });

  it("uses progress max of one when total is zero", () => {
    render(
      <ExportDialog
        state={{
          ...baseState,
          destination: "/tmp/out",
          jobId: "job-1",
          progress: {
            done: 0,
            total: 0,
            phase: "running",
            message: "Starting",
          },
        }}
        onClose={vi.fn()}
        onUpdate={vi.fn()}
        onStart={vi.fn()}
      />,
    );
    expect(screen.getByRole("progressbar")).toHaveAttribute("max", "1");
  });
});
