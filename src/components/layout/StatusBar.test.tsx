import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { errorNotification, successNotification } from "../../lib/notification";
import { StatusBar } from "./StatusBar";

describe("StatusBar", () => {
  it("shows library total and resolved status", () => {
    render(
      <StatusBar
        total={42}
        selectedCount={0}
        scanStatus=""
        roots={[]}
        exportActive={false}
        exportProgress={null}
        notification={null}
        busy={false}
      />,
    );

    expect(screen.getByText("42")).toBeInTheDocument();
    expect(screen.getByText("Ready")).toBeInTheDocument();
  });

  it("shows error notification text", () => {
    render(
      <StatusBar
        total={0}
        selectedCount={0}
        scanStatus=""
        roots={[]}
        exportActive={false}
        exportProgress={null}
        notification={errorNotification("disk full")}
        busy={false}
      />,
    );

    expect(screen.getByText("disk full")).toBeInTheDocument();
  });

  it("calls onDismissAlert when error dismiss is clicked", async () => {
    const user = userEvent.setup();
    const onDismissAlert = vi.fn();
    render(
      <StatusBar
        total={0}
        selectedCount={0}
        scanStatus=""
        roots={[]}
        exportActive={false}
        exportProgress={null}
        notification={errorNotification("disk full")}
        busy={false}
        onDismissAlert={onDismissAlert}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(onDismissAlert).toHaveBeenCalledOnce();
  });

  it("shows success styling for flash notifications", () => {
    render(
      <StatusBar
        total={0}
        selectedCount={0}
        scanStatus=""
        roots={[]}
        exportActive={false}
        exportProgress={null}
        notification={successNotification("Tagged 2 item(s)")}
        busy={false}
      />,
    );
    expect(screen.getByText("Tagged 2 item(s)").closest(".stat-value")).toHaveClass(
      "text-success",
    );
  });

  it("shows spinner during active progress", () => {
    render(
      <StatusBar
        total={0}
        selectedCount={0}
        scanStatus="indexing: 1/5"
        roots={[]}
        exportActive={false}
        exportProgress={null}
        notification={null}
        busy={false}
      />,
    );
    expect(document.querySelector(".loading-spinner")).toBeInTheDocument();
  });

  it("omits dismiss button without onDismissAlert", () => {
    render(
      <StatusBar
        total={0}
        selectedCount={0}
        scanStatus=""
        roots={[]}
        exportActive={false}
        exportProgress={null}
        notification={errorNotification("disk full")}
        busy={false}
      />,
    );
    expect(screen.queryByRole("button", { name: "Close" })).not.toBeInTheDocument();
  });
});
