import { describe, expect, it } from "vitest";
import { workspaceContextLabel } from "./workspaceContext";
import type { RecentWorkspace } from "../types";

function workspace(overrides: Partial<RecentWorkspace> = {}): RecentWorkspace {
  return {
    path: "/tmp/demo",
    name: "Demo",
    last_opened: 0,
    valid: true,
    root_count: 0,
    album_count: 0,
    tag_count: 0,
    read_only: false,
    ...overrides,
  };
}

describe("workspaceContextLabel", () => {
  it("formats linked counts with singular labels", () => {
    expect(
      workspaceContextLabel(
        workspace({ root_count: 1, album_count: 1, tag_count: 1 }),
      ),
    ).toBe("1 Library · 1 Album · 1 Tag");
  });

  it("formats linked counts with plural labels", () => {
    expect(
      workspaceContextLabel(
        workspace({ root_count: 2, album_count: 3, tag_count: 12 }),
      ),
    ).toBe("2 Libraries · 3 Albums · 12 Tags");
  });

  it("treats missing counts as zero", () => {
    expect(
      workspaceContextLabel(
        workspace({
          root_count: undefined as unknown as number,
          album_count: undefined as unknown as number,
          tag_count: undefined as unknown as number,
        }),
      ),
    ).toBe("0 Libraries · 0 Albums · 0 Tags");
  });
});
