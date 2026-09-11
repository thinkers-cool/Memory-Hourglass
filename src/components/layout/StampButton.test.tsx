import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { EMPTY_STAMP_CONFIG } from "../../lib/stamp";
import { StampButton } from "./StampButton";

describe("StampButton", () => {
  it("opens config on click when not armed", async () => {
    const user = userEvent.setup();
    render(
      <StampButton
        compact={false}
        armed={false}
        configValid={false}
        config={EMPTY_STAMP_CONFIG}
        tags={[]}
        albums={[]}
        onDisarm={vi.fn()}
        onRatingChange={vi.fn()}
        onToggleTag={vi.fn()}
        onToggleAlbum={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Stamp" }));
    expect(
      screen.getByRole("dialog", { name: "Stamp configuration" }),
    ).toBeInTheDocument();
  });

  it("disarms on click when armed", async () => {
    const user = userEvent.setup();
    const onDisarm = vi.fn();
    render(
      <StampButton
        compact={false}
        armed
        configValid
        config={{ ...EMPTY_STAMP_CONFIG, rating: 3 }}
        tags={[]}
        albums={[]}
        onDisarm={onDisarm}
        onRatingChange={vi.fn()}
        onToggleTag={vi.fn()}
        onToggleAlbum={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Stamp" }));
    expect(onDisarm).toHaveBeenCalledTimes(1);
    expect(
      screen.queryByRole("dialog", { name: "Stamp configuration" }),
    ).not.toBeInTheDocument();
  });

  it("shows space hint when armed", () => {
    render(
      <StampButton
        compact={false}
        armed
        configValid
        config={{ ...EMPTY_STAMP_CONFIG, rating: 3 }}
        tags={[]}
        albums={[]}
        onDisarm={vi.fn()}
        onRatingChange={vi.fn()}
        onToggleTag={vi.fn()}
        onToggleAlbum={vi.fn()}
      />,
    );

    expect(screen.getByText("Space")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stamp" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("closes config on outside click", async () => {
    const user = userEvent.setup();
    render(
      <>
        <StampButton
          compact={false}
          armed={false}
          configValid={false}
          config={EMPTY_STAMP_CONFIG}
          tags={[]}
          albums={[]}
          onDisarm={vi.fn()}
          onRatingChange={vi.fn()}
          onToggleTag={vi.fn()}
          onToggleAlbum={vi.fn()}
        />
        <button type="button">Outside</button>
      </>,
    );

    await user.click(screen.getByRole("button", { name: "Stamp" }));
    expect(
      screen.getByRole("dialog", { name: "Stamp configuration" }),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(
      screen.queryByRole("dialog", { name: "Stamp configuration" }),
    ).not.toBeInTheDocument();
  });

  it("renders compact mode armed and disarmed", async () => {
    const user = userEvent.setup();
    const onDisarm = vi.fn();
    const { rerender } = render(
      <StampButton
        compact
        armed={false}
        configValid={false}
        config={EMPTY_STAMP_CONFIG}
        tags={[]}
        albums={[]}
        onDisarm={onDisarm}
        onRatingChange={vi.fn()}
        onToggleTag={vi.fn()}
        onToggleAlbum={vi.fn()}
      />,
    );
    expect(screen.queryByText("Stamp")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Stamp" }));
    expect(
      screen.getByRole("dialog", { name: "Stamp configuration" }),
    ).toBeInTheDocument();

    rerender(
      <StampButton
        compact
        armed
        configValid
        config={{ ...EMPTY_STAMP_CONFIG, rating: 3 }}
        tags={[]}
        albums={[]}
        onDisarm={onDisarm}
        onRatingChange={vi.fn()}
        onToggleTag={vi.fn()}
        onToggleAlbum={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Stamp" }));
    expect(onDisarm).toHaveBeenCalledTimes(1);
  });

  it("does not open config when armed", async () => {
    const user = userEvent.setup();
    render(
      <StampButton
        compact={false}
        armed
        configValid
        config={{ ...EMPTY_STAMP_CONFIG, rating: 3 }}
        tags={[]}
        albums={[]}
        onDisarm={vi.fn()}
        onRatingChange={vi.fn()}
        onToggleTag={vi.fn()}
        onToggleAlbum={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Stamp" }));
    expect(
      screen.queryByRole("dialog", { name: "Stamp configuration" }),
    ).not.toBeInTheDocument();
  });
});
