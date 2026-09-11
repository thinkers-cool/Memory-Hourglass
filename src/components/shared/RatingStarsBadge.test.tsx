import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { RatingStarsBadge } from "./RatingStarsBadge";

describe("RatingStarsBadge", () => {
  it("renders star characters for the rating value", () => {
    render(<RatingStarsBadge rating={3} />);
    expect(screen.getByLabelText("Rating 3")).toHaveTextContent("★★★");
    expect(screen.getByLabelText("Rating 3")).not.toHaveTextContent("3");
  });

  it("renders nothing for zero rating", () => {
    const { container } = render(<RatingStarsBadge rating={0} />);
    expect(container).toBeEmptyDOMElement();
  });
});
