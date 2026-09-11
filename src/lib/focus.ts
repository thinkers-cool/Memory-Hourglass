const TEXT_INPUT_TYPES = new Set([
  "text",
  "search",
  "number",
  "email",
  "password",
  "url",
  "tel",
]);

export function isTextEntryElement(element: HTMLElement): boolean {
  if (element.isContentEditable) return true;
  const tag = element.tagName;
  if (tag === "TEXTAREA" || tag === "SELECT") return true;
  if (tag === "INPUT") {
    const type = (element as HTMLInputElement).type || "text";
    return TEXT_INPUT_TYPES.has(type);
  }
  return false;
}
