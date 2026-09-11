import i18n from "../../../i18n";
import { infoNotification } from "../../../lib/notification";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createGalleryActions(deps: LibraryActionsDeps) {
  const { items, selectedId, setGalleryIndex, setNotification } = deps;

  return {
    openGallery: () => {
      if (items.length === 0) {
        setNotification(
          infoNotification(i18n.t("library:notification.noSlideshowItems")),
        );
        return;
      }
      const idx =
        selectedId !== null ? items.findIndex((i) => i.id === selectedId) : 0;
      const nextIndex = idx >= 0 ? idx : 0;
      const open = () => setGalleryIndex(nextIndex);
      if (document.startViewTransition) {
        document.startViewTransition(open);
      } else {
        open();
      }
    },
  };
}
