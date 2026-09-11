import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { ThemeSwitcher } from "./ThemeSwitcher";
import { readStoredTheme, setTheme } from "../../lib/theme";

afterEach(() => {
  localStorage.clear();
  setTheme("slate");
});

describe("ThemeSwitcher", () => {
  it("lists themes and persists selection", async () => {
    const user = userEvent.setup();
    render(<ThemeSwitcher />);
    await user.click(screen.getByRole("button", { name: "Theme" }));
    const neon = screen.getByRole("radio", { name: "Neon" });
    await user.click(neon);
    expect(document.documentElement.getAttribute("data-theme")).toBe("neon");
    expect(readStoredTheme()).toBe("neon");
  });

  it("renders label mode and closes on outside click", async () => {
    const user = userEvent.setup();
    render(
      <>
        <ThemeSwitcher
          iconOnly={false}
          size="nav"
          menuPlacement="float-top-end"
        />
        <button type="button">Outside</button>
      </>,
    );

    expect(screen.getByText("Slate")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Theme" }));
    expect(screen.getByRole("radio", { name: "Neon" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(
      screen.queryByRole("radio", { name: "Neon" }),
    ).not.toBeInTheDocument();
  });

  it("positions menu using fallback dimensions before measure", async () => {
    const user = userEvent.setup();
    render(<ThemeSwitcher menuPlacement="float-bottom-end" />);
    await user.click(screen.getByRole("button", { name: "Theme" }));
    window.dispatchEvent(new Event("scroll"));
    expect(screen.getByRole("radio", { name: "Neon" })).toBeInTheDocument();
  });

  it("positions menu using measured dimensions", async () => {
    const user = userEvent.setup();
    render(<ThemeSwitcher />);
    await user.click(screen.getByRole("button", { name: "Theme" }));
    await waitFor(() => {
      expect(document.body.querySelector(".surface-popover")).toBeTruthy();
    });
    const menu = document.body.querySelector(".surface-popover") as HTMLElement;
    Object.defineProperty(menu, "offsetWidth", {
      configurable: true,
      value: 220,
    });
    Object.defineProperty(menu, "offsetHeight", {
      configurable: true,
      value: 280,
    });
    await act(async () => {
      window.dispatchEvent(new Event("resize"));
    });
    expect(screen.getByRole("radio", { name: "Neon" })).toBeInTheDocument();
  });
});
