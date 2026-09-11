import { useRef, useState } from "react";
import { Stamp } from "lucide-react";
import { useTranslation } from "react-i18next";
import { StampMegaDropdown } from "../shared/StampMegaDropdown";
import { usePopoverDismiss } from "../../hooks/usePopoverDismiss";
import { ghostBtnClass } from "../../lib/buttonClass";
import type { Album, TagDto } from "../../types";
import type { StampConfig } from "../../lib/stamp";
import { ShortcutHint } from "../shared/ShortcutHint";

export function StampButton({
  compact,
  armed,
  config,
  tags,
  albums,
  onDisarm,
  onRatingChange,
  onToggleTag,
  onToggleAlbum,
}: {
  compact: boolean;
  armed: boolean;
  configValid: boolean;
  config: StampConfig;
  tags: TagDto[];
  albums: Album[];
  onDisarm: () => void;
  onRatingChange: (rating: number | null) => void;
  onToggleTag: (tagId: number, add: boolean) => void;
  onToggleAlbum: (albumId: number, add: boolean) => void;
}) {
  const { t } = useTranslation(["library", "common"]);
  const [configOpen, setConfigOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const closeConfig = () => setConfigOpen(false);

  usePopoverDismiss({
    open: configOpen,
    onClose: closeConfig,
    containerRef: panelRef,
    anchorRef: rootRef,
  });

  const buttonClass = compact
    ? armed
      ? "btn btn-primary btn-sm btn-square h-8 min-h-0 w-8"
      : ghostBtnClass("btn-sm btn-square h-8 min-h-0 w-8")
    : armed
      ? "btn btn-primary btn-sm h-8 min-h-0 gap-1 text-xs font-normal"
      : ghostBtnClass("btn-sm h-8 min-h-0 gap-1 text-xs font-normal");

  const handleClick = () => {
    if (armed) {
      onDisarm();
      closeConfig();
      return;
    }
    setConfigOpen((prev) => !prev);
  };

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        className={buttonClass}
        title={armed ? t("library:stamp.armed") : t("library:stamp.configure")}
        aria-label={t("common:aria.stamp")}
        aria-pressed={armed}
        aria-expanded={configOpen}
        onClick={handleClick}
      >
        <Stamp className="h-3.5 w-3.5" />
        {!compact ? (
          <>
            {t("library:stamp.label")}
            {armed ? (
              <ShortcutHint>{t("common:shortcut.space")}</ShortcutHint>
            ) : null}
          </>
        ) : null}
      </button>

      {configOpen && (
        <div ref={panelRef} className="absolute right-0 top-full z-[60] mt-1">
          <StampMegaDropdown
            config={config}
            tags={tags}
            albums={albums}
            onRatingChange={onRatingChange}
            onToggleTag={onToggleTag}
            onToggleAlbum={onToggleAlbum}
          />
        </div>
      )}
    </div>
  );
}
