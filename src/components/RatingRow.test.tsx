import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RatingRow } from "./RatingRow";

describe("RatingRow", () => {
  it("renders one star per slot and selects a value", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<RatingRow value={2} onSelect={onSelect} />);

    await user.click(screen.getByRole("button", { name: "Rate 4 stars" }));
    expect(onSelect).toHaveBeenCalledWith(4);
    expect(screen.getByRole("button", { name: "Rate 1 star" })).toHaveTextContent("★");
    expect(screen.getByRole("button", { name: "Rate 2 stars" })).toHaveTextContent("★");
    expect(screen.getByRole("button", { name: "Rate 3 stars" })).toHaveTextContent("☆");
    expect(screen.getByRole("button", { name: "Rate 4 stars" })).toHaveTextContent("☆");
  });

  it("fills stars up to the active rating in compact mode", () => {
    render(<RatingRow compact value={3} onSelect={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Rate 1 star" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Rate 3 stars" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Rate 4 stars" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    expect(screen.getByRole("button", { name: "Rate 1 star" })).toHaveTextContent("★");
    expect(screen.getByRole("button", { name: "Rate 5 stars" })).toHaveTextContent("☆");
  });
});
