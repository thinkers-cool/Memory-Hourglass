import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SortDropdown } from "./SortDropdown";

describe("SortDropdown", () => {
  it("changes sort mode", async () => {
    const user = userEvent.setup();
    const onSortChange = vi.fn();
    render(
      <SortDropdown
        sort="date"
        sortDir="desc"
        onSortChange={onSortChange}
        onSortDirChange={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Date/i }));
    await user.click(screen.getByRole("button", { name: "Name" }));
    expect(onSortChange).toHaveBeenCalledWith("name");
  });

  it("toggles sort direction", async () => {
    const user = userEvent.setup();
    const onSortDirChange = vi.fn();
    render(
      <SortDropdown
        sort="date"
        sortDir="desc"
        onSortChange={vi.fn()}
        onSortDirChange={onSortDirChange}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Descending/i }));
    expect(onSortDirChange).toHaveBeenCalledWith("asc");
  });

  it("renders compact mode without field label", () => {
    render(
      <SortDropdown
        compact
        sort="rating"
        sortDir="asc"
        onSortChange={vi.fn()}
        onSortDirChange={vi.fn()}
      />,
    );
    expect(screen.queryByText("Rating")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Sort by Rating/i }),
    ).toBeInTheDocument();
  });

  it("falls back to date for unknown sort mode", () => {
    render(
      <SortDropdown
        sort={"unknown" as "date"}
        sortDir="asc"
        onSortChange={vi.fn()}
        onSortDirChange={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button", { name: /Sort by Date/i }),
    ).toBeInTheDocument();
  });

  it("toggles from ascending to descending", async () => {
    const user = userEvent.setup();
    const onSortDirChange = vi.fn();
    render(
      <SortDropdown
        sort="date"
        sortDir="asc"
        onSortChange={vi.fn()}
        onSortDirChange={onSortDirChange}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Ascending/i }));
    expect(onSortDirChange).toHaveBeenCalledWith("desc");
  });

  it("closes menu on outside click", async () => {
    const user = userEvent.setup();
    render(
      <>
        <SortDropdown
          sort="date"
          sortDir="desc"
          onSortChange={vi.fn()}
          onSortDirChange={vi.fn()}
        />
        <button type="button">Outside</button>
      </>,
    );
    await user.click(screen.getByRole("button", { name: /Date/i }));
    expect(screen.getByRole("button", { name: "Name" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(
      screen.queryByRole("button", { name: "Name" }),
    ).not.toBeInTheDocument();
  });

  it("closes menu after selecting sort option", async () => {
    const user = userEvent.setup();
    render(
      <SortDropdown
        sort="date"
        sortDir="desc"
        onSortChange={vi.fn()}
        onSortDirChange={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Date/i }));
    await user.click(screen.getByRole("button", { name: "Path" }));
    expect(
      screen.queryByRole("button", { name: "Path" }),
    ).not.toBeInTheDocument();
  });
});
