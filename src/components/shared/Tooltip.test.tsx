import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { IconTooltip, Tooltip } from "./Tooltip";

function mockAnchorRect(rect: Partial<DOMRect>) {
  const value = {
    top: 100,
    left: 100,
    right: 200,
    bottom: 130,
    width: 100,
    height: 30,
    x: 100,
    y: 100,
    toJSON: () => ({}),
    ...rect,
  } as DOMRect;
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(
    value,
  );
}

function mockTipDimensions(width = 80, height = 24) {
  Object.defineProperty(HTMLElement.prototype, "offsetWidth", {
    configurable: true,
    get: () => width,
  });
  Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
    configurable: true,
    get: () => height,
  });
  Object.defineProperty(HTMLElement.prototype, "scrollWidth", {
    configurable: true,
    get: () => width,
  });
  Object.defineProperty(HTMLElement.prototype, "scrollHeight", {
    configurable: true,
    get: () => height,
  });
}

describe("Tooltip", () => {
  beforeEach(() => {
    vi.spyOn(window, "innerWidth", "get").mockReturnValue(1000);
    vi.spyOn(window, "innerHeight", "get").mockReturnValue(800);
    mockTipDimensions();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders children without wrapper when tip is empty", () => {
    render(
      <Tooltip tip="">
        <button type="button">Action</button>
      </Tooltip>,
    );

    expect(screen.queryByRole("tooltip")).toBeNull();
    expect(screen.getByRole("button", { name: "Action" })).toBeInTheDocument();
  });

  it("retries layout when tip dimensions are not ready", async () => {
    let rafCalls = 0;
    const rafSpy = vi
      .spyOn(window, "requestAnimationFrame")
      .mockImplementation((callback) => {
        rafCalls += 1;
        if (rafCalls === 1) {
          callback(0);
        } else {
          Object.defineProperty(HTMLElement.prototype, "scrollWidth", {
            configurable: true,
            get: () => 80,
          });
          Object.defineProperty(HTMLElement.prototype, "scrollHeight", {
            configurable: true,
            get: () => 24,
          });
          callback(0);
        }
        return 0;
      });
    Object.defineProperty(HTMLElement.prototype, "offsetWidth", {
      configurable: true,
      get: () => 0,
    });
    Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
      configurable: true,
      get: () => 0,
    });
    Object.defineProperty(HTMLElement.prototype, "scrollWidth", {
      configurable: true,
      get: () => 0,
    });
    Object.defineProperty(HTMLElement.prototype, "scrollHeight", {
      configurable: true,
      get: () => 0,
    });
    const user = userEvent.setup();
    render(
      <Tooltip tip="Delayed layout" placement="top">
        <button type="button">Delay</button>
      </Tooltip>,
    );
    await user.hover(screen.getByRole("button", { name: "Delay" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Delayed layout");
    expect(rafSpy).toHaveBeenCalled();
  });

  it("retries layout when only tip height is zero", async () => {
    const rafSpy = vi
      .spyOn(window, "requestAnimationFrame")
      .mockImplementation((callback) => {
        Object.defineProperty(HTMLElement.prototype, "scrollHeight", {
          configurable: true,
          get: () => 24,
        });
        callback(0);
        return 0;
      });
    Object.defineProperty(HTMLElement.prototype, "offsetWidth", {
      configurable: true,
      get: () => 80,
    });
    Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
      configurable: true,
      get: () => 0,
    });
    Object.defineProperty(HTMLElement.prototype, "scrollWidth", {
      configurable: true,
      get: () => 80,
    });
    Object.defineProperty(HTMLElement.prototype, "scrollHeight", {
      configurable: true,
      get: () => 0,
    });
    const user = userEvent.setup();
    render(
      <Tooltip tip="Height retry" placement="top">
        <button type="button">Height</button>
      </Tooltip>,
    );
    await user.hover(screen.getByRole("button", { name: "Height" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Height retry");
    expect(rafSpy).toHaveBeenCalled();
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

  it("repositions on scroll", async () => {
    mockAnchorRect({ left: 20, right: 120 });
    const user = userEvent.setup();
    render(
      <Tooltip tip="Near left edge" placement="auto">
        <button type="button">Edge</button>
      </Tooltip>,
    );

    await user.hover(screen.getByRole("button", { name: "Edge" }));
    const tooltip = await screen.findByRole("tooltip");
    expect(tooltip).toHaveTextContent("Near left edge");

    fireEvent.scroll(window);
    await waitFor(() => expect(tooltip).toBeVisible());
  });

  it("supports focus events", async () => {
    mockAnchorRect({ top: 20, bottom: 50 });
    render(
      <Tooltip tip="Top edge" placement="bottom">
        <button type="button">Focus me</button>
      </Tooltip>,
    );

    fireEvent.focus(screen.getByRole("button", { name: "Focus me" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Top edge");
    fireEvent.blur(screen.getByRole("button", { name: "Focus me" }));
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
  beforeEach(() => {
    mockTipDimensions();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

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
