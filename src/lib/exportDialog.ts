import type { ExportOptions } from "../types";
import type { ExportProgressState } from "./jobProgress";
import { DEFAULT_EXPORT_OPTIONS } from "./libraryActions";

export type ExportDialogOpenState = {
  open: boolean;
  assetIds: number[];
  destination: string;
  options: ExportOptions;
  jobId: number | null;
  progress: ExportProgressState;
};

export function createExportDialogState(
  assetIds: number[],
): ExportDialogOpenState {
  return {
    open: true,
    assetIds,
    destination: "",
    options: DEFAULT_EXPORT_OPTIONS,
    jobId: null,
    progress: null,
  };
}
