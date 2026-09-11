import { describe, expect, it } from "vitest";
import type { FilterDef } from "../components/shared/FilterChipBar";
import { getDisplayValue, getMultiDisplayValue } from "./filterChipDisplay";

const statusFilter: FilterDef = {
  id: "sync",
  label: "Sync",
  type: "status",
  statusOptions: ["ok", "new"],
  statusOptionLabels: { ok: "Ok", new: "New" },
};

const dateFilter: FilterDef = {
  id: "capture",
  label: "Date",
  type: "date",
};

const textFilter: FilterDef = {
  id: "metadata",
  label: "Metadata",
  type: "text",
};

describe("getMultiDisplayValue", () => {
  it("returns empty string for no values", () => {
    expect(getMultiDisplayValue(statusFilter, [])).toBe("");
  });

  it("joins one or two values", () => {
    expect(getMultiDisplayValue(statusFilter, ["ok"])).toBe("Ok");
    expect(getMultiDisplayValue(statusFilter, ["ok", "new"])).toBe("Ok, New");
  });

  it("summarizes more than two values", () => {
    expect(getMultiDisplayValue(statusFilter, ["ok", "new", "ok"])).toBe(
      "Ok, +2",
    );
  });

  it("falls back to raw values without option labels", () => {
    const rawFilter: FilterDef = {
      id: "sync",
      label: "Sync",
      type: "status",
      statusOptions: ["ok", "new"],
    };
    expect(getMultiDisplayValue(rawFilter, ["ok", "new"])).toBe("ok, new");
  });
});

describe("getDisplayValue", () => {
  it("formats date ranges", () => {
    expect(getDisplayValue(dateFilter, "", "2024-01-01", "2024-12-31")).toBe(
      "2024-01-01 – 2024-12-31",
    );
    expect(getDisplayValue(dateFilter, "", "2024-01-01", "")).toBe(
      "from 2024-01-01",
    );
    expect(getDisplayValue(dateFilter, "", "", "2024-12-31")).toBe(
      "to 2024-12-31",
    );
    expect(getDisplayValue(dateFilter, "", "", "")).toBe("");
  });

  it("maps status labels and raw text values", () => {
    expect(getDisplayValue(statusFilter, "ok", "", "")).toBe("Ok");
    expect(getDisplayValue(textFilter, "sunset", "", "")).toBe("sunset");
  });

  it("falls back to raw status values without labels", () => {
    const rawFilter: FilterDef = {
      id: "sync",
      label: "Sync",
      type: "status",
      statusOptions: ["ok"],
    };
    expect(getDisplayValue(rawFilter, "ok", "", "")).toBe("ok");
  });
});
