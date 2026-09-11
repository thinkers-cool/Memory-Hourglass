import type { JobPhase, JobProgress } from "../types";

export type ExportProgressState = {
  done: number;
  total: number;
  phase: JobPhase;
  message: string;
  file_name?: string;
} | null;

export function isJobFinished(phase: JobPhase): boolean {
  return phase === "completed" || phase === "failed";
}

export function toExportProgress(progress: JobProgress): ExportProgressState {
  return {
    done: progress.done,
    total: progress.total,
    phase: progress.phase,
    message: progress.message,
    file_name: progress.file_name,
  };
}

export function exportProgressLabel(
  progress: NonNullable<ExportProgressState>,
): string {
  if (isJobFinished(progress.phase)) {
    return progress.message;
  }
  if (progress.file_name) {
    return `${progress.message}: ${progress.file_name}`;
  }
  return progress.message;
}
