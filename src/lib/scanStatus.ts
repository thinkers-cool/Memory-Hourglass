export type ScanProgressPhase = "scanning" | "cataloging" | "indexing";

export type ScanProgress = {
  phase: ScanProgressPhase;
  done: number | null;
  total: number | null;
};

const SCAN_PROGRESS_PHASES: ScanProgressPhase[] = [
  "scanning",
  "cataloging",
  "indexing",
];

export function parseScanProgress(scanStatus: string): ScanProgress | null {
  for (const phase of SCAN_PROGRESS_PHASES) {
    const prefix = `${phase}:`;
    if (!scanStatus.startsWith(prefix)) continue;
    const remainder = scanStatus.slice(prefix.length).trim();
    if (!remainder) {
      return { phase, done: null, total: null };
    }
    const match = remainder.match(/^(\d+)\/(\d+)$/);
    if (!match) {
      return { phase, done: null, total: null };
    }
    return {
      phase,
      done: Number(match[1]),
      total: Number(match[2]),
    };
  }
  return null;
}

export function scanProgressCounts(progress: ScanProgress): string {
  if (progress.done === null || progress.total === null) {
    return "";
  }
  return `${progress.done}/${progress.total}`;
}

export function resolveRootScanProgress(
  scanStatus?: string,
): ScanProgress | null {
  if (
    !scanStatus ||
    scanStatus === "" ||
    scanStatus.startsWith("done:") ||
    scanStatus.startsWith("error:")
  ) {
    return null;
  }
  return parseScanProgress(scanStatus);
}
