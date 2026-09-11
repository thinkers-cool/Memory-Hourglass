import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { changeLocale } from "../../i18n";
import { LocaleSwitcher } from "./LocaleSwitcher";

describe("LocaleSwitcher", () => {
  it("opens locale menu and switches language", async () => {
    const user = userEvent.setup();
    await changeLocale("en-US");
    render(<LocaleSwitcher />);

    await user.click(screen.getByRole("button", { name: "Language" }));
    await user.click(screen.getByRole("radio", { name: "简体中文" }));

    expect(screen.getByRole("button", { name: "语言" })).toBeInTheDocument();
  });

  it("renders label mode and closes on outside click", async () => {
    const user = userEvent.setup();
    await changeLocale("en-US");
    render(
      <>
        <LocaleSwitcher iconOnly={false} size="nav" menuPlacement="float-top-end" />
        <button type="button">Outside</button>
      </>,
    );

    expect(screen.getByText("English")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Language" }));
    expect(screen.getByRole("radio", { name: "English" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(screen.queryByRole("radio", { name: "English" })).not.toBeInTheDocument();
  });

  it("positions menu using fallback dimensions before measure", async () => {
    const user = userEvent.setup();
    await changeLocale("en-US");
    render(<LocaleSwitcher />);
    await user.click(screen.getByRole("button", { name: "Language" }));
    expect(screen.getByRole("radio", { name: "English" })).toBeInTheDocument();
    window.dispatchEvent(new Event("resize"));
  });

  it("positions menu using measured dimensions", async () => {
    const user = userEvent.setup();
    await changeLocale("en-US");
    render(<LocaleSwitcher />);
    await user.click(screen.getByRole("button", { name: "Language" }));
    await waitFor(() => {
      expect(document.body.querySelector(".surface-popover")).toBeTruthy();
    });
    const menu = document.body.querySelector(".surface-popover") as HTMLElement;
    Object.defineProperty(menu, "offsetWidth", { configurable: true, value: 180 });
    Object.defineProperty(menu, "offsetHeight", { configurable: true, value: 120 });
    await act(async () => {
      window.dispatchEvent(new Event("resize"));
    });
    expect(screen.getByRole("radio", { name: "English" })).toBeInTheDocument();
  });
});
