import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TagColorDot } from "./TagColorDot";

describe("TagColorDot", () => {
  it("renders normalized color", () => {
    const { container } = render(<TagColorDot color="#336699" />);
    const dot = container.querySelector("span");
    expect(dot).toHaveStyle({ backgroundColor: "#336699" });
  });
});
