import { isActiveScan } from "./statusBar";

export function pickActiveScanStatus(
  scanStatusByRoot: Record<number, string>,
): string {
  for (const status of Object.values(scanStatusByRoot)) {
    if (isActiveScan(status)) {
      return status;
    }
  }
  return "";
}
