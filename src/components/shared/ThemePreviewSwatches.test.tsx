import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ThemePreviewSwatches } from "./ThemePreviewSwatches";

describe("ThemePreviewSwatches", () => {
  it("renders explicit preview colors", () => {
    const { container } = render(
      <ThemePreviewSwatches
        theme="slate"
        colors={{
          background: "#111111",
          primary: "#222222",
          strong: "#333333",
        }}
      />,
    );
    const swatches = container.querySelectorAll("span[style*='background-color']");
    expect(swatches).toHaveLength(3);
    expect(swatches[0]).toHaveStyle({ backgroundColor: "#111111" });
    expect(swatches[1]).toHaveStyle({ backgroundColor: "#222222" });
    expect(swatches[2]).toHaveStyle({ backgroundColor: "#333333" });
  });

  it("renders theme variable swatches when colors are omitted", () => {
    const { container } = render(<ThemePreviewSwatches theme="neon" />);
    const themed = container.querySelector("[data-theme='neon']");
    expect(themed).toBeInTheDocument();
    expect(themed?.querySelectorAll("span[style]")).toHaveLength(3);
  });
});
