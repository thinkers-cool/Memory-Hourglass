import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DateRangeFilter } from "./DateRangeFilter";

describe("DateRangeFilter", () => {
  it("updates and clears date range", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <DateRangeFilter dateGte="2024-01-01" dateLte="" onChange={onChange} />,
    );
    await user.click(screen.getByTitle("Clear dates"));
    expect(onChange).toHaveBeenCalledWith("", "");
  });

  it("updates from date", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <DateRangeFilter dateGte="" dateLte="" onChange={onChange} />,
    );
    await user.type(screen.getByTitle("From"), "2024-06-01");
    expect(onChange).toHaveBeenCalled();
  });

  it("updates to date", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <DateRangeFilter dateGte="2024-01-01" dateLte="" onChange={onChange} />,
    );
    await user.type(screen.getByTitle("To"), "2024-12-31");
    expect(onChange).toHaveBeenCalled();
  });

  it("hides clear button when both dates are empty", () => {
    render(
      <DateRangeFilter dateGte="" dateLte="" onChange={vi.fn()} />,
    );
    expect(screen.queryByTitle("Clear dates")).not.toBeInTheDocument();
  });

  it("renders xs size inputs", () => {
    render(
      <DateRangeFilter dateGte="" dateLte="" onChange={vi.fn()} size="xs" />,
    );
    expect(screen.getByTitle("From")).toHaveClass("input-xs");
  });
});
