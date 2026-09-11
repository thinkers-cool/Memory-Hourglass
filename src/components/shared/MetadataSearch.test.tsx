import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MetadataFieldSearch, useFilteredRawTags } from "./MetadataSearch";
import type { RawTag } from "../../types";

const tags: RawTag[] = [
  { name: "ISO", value: "400" },
  { name: "LensModel", value: "35mm" },
];

describe("useFilteredRawTags", () => {
  it("filters tags through the hook", () => {
    const { result, rerender } = renderHook(
      ({ query }) => useFilteredRawTags(tags, query),
      { initialProps: { query: "" } },
    );
    expect(result.current).toHaveLength(2);

    rerender({ query: "lens" });
    expect(result.current).toEqual([tags[1]]);
  });
});

describe("MetadataFieldSearch", () => {
  it("updates value on input", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<MetadataFieldSearch value="" onChange={onChange} />);
    await user.type(screen.getByRole("textbox"), "iso");
    expect(onChange).toHaveBeenCalled();
  });

  it("clears value with button and Escape", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<MetadataFieldSearch value="iso" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Clear all" }));
    expect(onChange).toHaveBeenCalledWith("");

    onChange.mockClear();
    screen.getByRole("textbox").focus();
    await user.keyboard("{Escape}");
    expect(onChange).toHaveBeenCalledWith("");
  });

  it("uses custom placeholder", () => {
    render(
      <MetadataFieldSearch
        value=""
        onChange={vi.fn()}
        placeholder="Search fields"
      />,
    );
    expect(screen.getByPlaceholderText("Search fields")).toBeInTheDocument();
  });
});
