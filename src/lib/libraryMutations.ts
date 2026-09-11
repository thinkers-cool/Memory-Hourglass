import * as api from "../api/client";

export async function runTagToggle(
  assetIds: number[],
  tagId: number,
  add: boolean,
): Promise<number> {
  return add
    ? await api.batchAppendTags(assetIds, tagId)
    : await api.batchRemoveTags(assetIds, tagId);
}

export async function runTagCreate(
  assetIds: number[],
  tagName: string,
): Promise<number> {
  const trimmed = tagName.trim();
  if (!trimmed) return 0;
  const tag = await api.createTag(trimmed);
  await api.batchAppendTags(assetIds, tag.id);
  return tag.id;
}

export async function runAlbumToggle(
  assetIds: number[],
  albumId: number,
  add: boolean,
): Promise<number> {
  return add
    ? await api.addAlbumItems(albumId, assetIds)
    : await api.removeAlbumItems(albumId, assetIds);
}

export async function runAlbumCreate(
  assetIds: number[],
  name: string,
): Promise<number> {
  const trimmed = name.trim();
  if (!trimmed) return 0;
  const album = await api.createAlbum(trimmed);
  await api.addAlbumItems(album.id, assetIds);
  return album.id;
}
