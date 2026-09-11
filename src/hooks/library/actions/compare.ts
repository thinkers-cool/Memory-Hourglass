import * as api from "../../../api/client";
import i18n from "../../../i18n";
import {
  runAlbumCreate,
  runAlbumToggle,
  runTagCreate,
  runTagToggle,
} from "../../../lib/libraryMutations";
import { infoNotification } from "../../../lib/notification";
import type { LibraryActionsDeps } from "../libraryActionsDeps";

export function createCompareActions(deps: LibraryActionsDeps) {
  const {
    selectedList,
    sortParam,
    setCompareOpen,
    setCompareIds,
    setCompareItems,
    setCompareDetails,
    setNotification,
    refreshGrid,
    refreshMeta,
    withBusy,
  } = deps;

  return {
    openCompare: async () => {
      if (selectedList.length < 2) {
        setNotification(
          infoNotification(i18n.t("library:notification.selectTwoToCompare")),
        );
        return;
      }
      setCompareIds(selectedList);
      setCompareOpen(true);
      const result = await api.queryAssets(
        { asset_ids: selectedList },
        sortParam,
        0,
        selectedList.length,
      );
      setCompareItems(result.items);
      const details = await Promise.all(
        selectedList.map((id) => api.getAsset(id)),
      );
      setCompareDetails(
        Object.fromEntries(
          details.map((assetDetail) => [
            assetDetail.asset.id,
            {
              tag_ids: assetDetail.tag_ids,
              album_ids: assetDetail.album_ids,
            },
          ]),
        ),
      );
    },
    closeCompare: () => {
      setCompareOpen(false);
      setCompareIds([]);
      setCompareItems([]);
      setCompareDetails({});
    },
    rateAsset: async (id: number, rating: number) => {
      await api.updateAssetMeta(id, { rating });
      await refreshGrid();
    },
    toggleTagOnAsset: async (id: number, tagId: number, add: boolean) => {
      await withBusy(async () => {
        await runTagToggle([id], tagId, add);
        await refreshGrid();
        await refreshMeta();
        const assetDetail = await api.getAsset(id);
        setCompareDetails((prev) => ({
          ...prev,
          [id]: {
            tag_ids: assetDetail.tag_ids,
            album_ids: assetDetail.album_ids,
          },
        }));
      });
    },
    createTagOnAsset: async (id: number, tagName: string) => {
      const trimmed = tagName.trim();
      if (!trimmed) return;
      let tagId = 0;
      await withBusy(async () => {
        tagId = await runTagCreate([id], trimmed);
        await refreshMeta();
        await refreshGrid();
        const assetDetail = await api.getAsset(id);
        setCompareDetails((prev) => ({
          ...prev,
          [id]: {
            tag_ids: assetDetail.tag_ids,
            album_ids: assetDetail.album_ids,
          },
        }));
      });
      return tagId > 0 ? String(tagId) : undefined;
    },
    toggleAlbumOnAsset: async (id: number, albumId: number, add: boolean) => {
      await withBusy(async () => {
        await runAlbumToggle([id], albumId, add);
        await refreshMeta();
        const assetDetail = await api.getAsset(id);
        setCompareDetails((prev) => ({
          ...prev,
          [id]: {
            tag_ids: assetDetail.tag_ids,
            album_ids: assetDetail.album_ids,
          },
        }));
      });
    },
    createAlbumOnAsset: async (id: number, name: string) => {
      const trimmed = name.trim();
      if (!trimmed) return;
      let albumId = 0;
      await withBusy(async () => {
        albumId = await runAlbumCreate([id], trimmed);
        await refreshMeta();
        const assetDetail = await api.getAsset(id);
        setCompareDetails((prev) => ({
          ...prev,
          [id]: {
            tag_ids: assetDetail.tag_ids,
            album_ids: assetDetail.album_ids,
          },
        }));
      });
      return albumId > 0 ? String(albumId) : undefined;
    },
  };
}
