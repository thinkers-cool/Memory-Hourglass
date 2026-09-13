import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
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
});
