import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { IconTooltip, Tooltip } from "./Tooltip";

describe("Tooltip", () => {
  it("renders children without wrapper when tip is empty", () => {
    render(
      <Tooltip tip="">
        <button type="button">Action</button>
      </Tooltip>,
    );

    expect(screen.queryByRole("tooltip")).toBeNull();
    expect(screen.getByRole("button", { name: "Action" })).toBeInTheDocument();
  });

  it("renders a portal tooltip on hover", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip tip="Sync folder" placement="bottom">
        <button type="button">Action</button>
      </Tooltip>,
    );

    await user.hover(screen.getByRole("button", { name: "Action" }));

    expect(await screen.findByRole("tooltip")).toHaveTextContent("Sync folder");
  });

  it("renders multiline path tips", async () => {
    const user = userEvent.setup();
    const path = "/Users/tony/Photos";
    render(
      <Tooltip tip={path} multiline>
        <span>Photos</span>
      </Tooltip>,
    );

    await user.hover(screen.getByText("Photos"));

    expect(await screen.findByRole("tooltip")).toHaveTextContent(path);
  });
});

describe("IconTooltip", () => {
  it("shows the tooltip after the shared hover delay", async () => {
    const user = userEvent.setup();
    render(
      <IconTooltip tip="Remove">
        <button type="button">X</button>
      </IconTooltip>,
    );

    await user.hover(screen.getByRole("button", { name: "X" }));

    expect(screen.queryByRole("tooltip")).toBeNull();

    expect(await screen.findByRole("tooltip")).toHaveTextContent("Remove");
  }, 10_000);
});
