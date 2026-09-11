import { describe, expect, it } from "vitest";
import { DEFAULT_SORT_DIR, encodeSortParam } from "./sortSettings";

describe("sortSettings", () => {
  it("encodes sort field and direction", () => {
    expect(encodeSortParam("name", "asc")).toBe("name:asc");
    expect(encodeSortParam("date", "desc")).toBe("date:desc");
    expect(encodeSortParam("rating", "desc")).toBe("rating:desc");
    expect(encodeSortParam("path", "asc")).toBe("path:asc");
  });

  it("uses sensible defaults per field", () => {
    expect(DEFAULT_SORT_DIR.date).toBe("desc");
    expect(DEFAULT_SORT_DIR.name).toBe("asc");
    expect(DEFAULT_SORT_DIR.rating).toBe("desc");
    expect(DEFAULT_SORT_DIR.path).toBe("asc");
  });
});
