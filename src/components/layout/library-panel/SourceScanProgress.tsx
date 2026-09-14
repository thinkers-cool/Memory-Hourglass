import { useTranslation } from "react-i18next";
import type { ScanProgress } from "../../../lib/scanStatus";

export function SourceScanProgress({ progress }: { progress: ScanProgress }) {
  const { t } = useTranslation("library");
  const counts =
    progress.done !== null && progress.total !== null
      ? `${progress.done}/${progress.total}`
      : null;

  return (
    <span className="inline-flex items-center gap-1 font-medium opacity-70">
      <span>{t(`panel.scanPhase.${progress.phase}`)}</span>
      {counts ? <span className="tabular-nums">{counts}</span> : null}
    </span>
  );
}
