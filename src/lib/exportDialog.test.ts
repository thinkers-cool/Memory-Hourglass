import { describe, expect, it } from "vitest";
import { createExportDialogState } from "./exportDialog";
import { DEFAULT_EXPORT_OPTIONS } from "./libraryActions";

describe("createExportDialogState", () => {
  it("opens dialog with asset ids and defaults", () => {
    expect(createExportDialogState([1, 2])).toEqual({
      open: true,
      assetIds: [1, 2],
      destination: "",
      options: DEFAULT_EXPORT_OPTIONS,
      jobId: null,
      progress: null,
    });
  });
});
