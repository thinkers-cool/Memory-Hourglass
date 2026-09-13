import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SectionOrderButton } from "./SectionOrderButton";

describe("SectionOrderButton", () => {
  it("renders ascending and descending states", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    const { rerender } = render(
      <SectionOrderButton mode="name-asc" busy={false} onToggle={onToggle} />,
    );

    await user.click(screen.getByRole("button"));
    expect(onToggle).toHaveBeenCalledTimes(1);

    rerender(
      <SectionOrderButton mode="name-desc" busy={true} onToggle={onToggle} />,
    );
    expect(screen.getByRole("button")).toBeDisabled();
  });
});
