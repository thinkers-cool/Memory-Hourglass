import { describe, expect, it } from "vitest";
import { isTextEntryElement } from "./focus";

function mockElement(
  tagName: string,
  options: { type?: string; contentEditable?: boolean } = {},
): HTMLElement {
  return {
    tagName: tagName.toUpperCase(),
    isContentEditable: options.contentEditable ?? false,
    type: options.type,
  } as unknown as HTMLElement;
}

describe("isTextEntryElement", () => {
  it("detects content editable elements", () => {
    expect(
      isTextEntryElement(mockElement("div", { contentEditable: true })),
    ).toBe(true);
  });

  it("detects textarea and select elements", () => {
    expect(isTextEntryElement(mockElement("textarea"))).toBe(true);
    expect(isTextEntryElement(mockElement("select"))).toBe(true);
  });

  it("detects text-like input types", () => {
    expect(isTextEntryElement(mockElement("input", { type: "text" }))).toBe(
      true,
    );
    expect(isTextEntryElement(mockElement("input", { type: "search" }))).toBe(
      true,
    );
    expect(isTextEntryElement(mockElement("input", { type: "number" }))).toBe(
      true,
    );
    expect(isTextEntryElement(mockElement("input", { type: "email" }))).toBe(
      true,
    );
    expect(isTextEntryElement(mockElement("input", { type: "password" }))).toBe(
      true,
    );
    expect(isTextEntryElement(mockElement("input", { type: "url" }))).toBe(
      true,
    );
    expect(isTextEntryElement(mockElement("input", { type: "tel" }))).toBe(
      true,
    );
  });

  it("treats missing input type as text", () => {
    expect(isTextEntryElement(mockElement("input"))).toBe(true);
  });

  it("rejects non-text inputs and generic elements", () => {
    expect(isTextEntryElement(mockElement("input", { type: "checkbox" }))).toBe(
      false,
    );
    expect(isTextEntryElement(mockElement("button"))).toBe(false);
    expect(isTextEntryElement(mockElement("div"))).toBe(false);
  });
});
