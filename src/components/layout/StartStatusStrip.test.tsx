import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { errorNotification } from "../../lib/notification";
import { StartStatusStrip } from "./StartStatusStrip";

describe("StartStatusStrip", () => {
  it("shows busy message", () => {
    render(
      <StartStatusStrip
        busy
        busyMessage="Opening workspace…"
        notification={null}
      />,
    );
    expect(screen.getByText("Opening workspace…")).toBeInTheDocument();
  });

  it("shows error notification", () => {
    render(
      <StartStatusStrip
        busy={false}
        busyMessage=""
        notification={errorNotification("disk full")}
      />,
    );
    expect(screen.getByRole("alert")).toHaveTextContent("disk full");
  });

  it("hides when idle with no notification", () => {
    const { container } = render(
      <StartStatusStrip busy={false} busyMessage="" notification={null} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("calls onDismissAlert when error dismiss is clicked", async () => {
    const user = userEvent.setup();
    const onDismissAlert = vi.fn();
    render(
      <StartStatusStrip
        busy={false}
        busyMessage=""
        notification={errorNotification("disk full")}
        onDismissAlert={onDismissAlert}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(onDismissAlert).toHaveBeenCalledOnce();
  });
});
