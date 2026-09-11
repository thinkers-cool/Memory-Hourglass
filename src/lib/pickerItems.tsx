import type { Album, TagDto } from "../types";
import { albumEmoji } from "./libraryIndicators";
import { tagDepthFirst, tagPathLabel } from "./tagHierarchy";
import type { PickerItem } from "../components/shared/SelectionPickerPopover";
import { TagColorDot } from "../components/shared/TagColorDot";

export function buildTagPickerItems(tags: TagDto[]): PickerItem[] {
  return tagDepthFirst(tags).map((tag) => ({
    key: String(tag.id),
    label: tagPathLabel(tag, tags),
    icon: <TagColorDot color={tag.color} />,
  }));
}

export function buildAlbumPickerItems(albums: Album[]): PickerItem[] {
  return albums.map((album) => ({
    key: String(album.id),
    label: album.name,
    icon: (
      <span className="text-sm leading-none">{albumEmoji(album.emoji)}</span>
    ),
  }));
}

export function ToolbarDivider() {
  return <div className="mx-1 h-5 w-px shrink-0 bg-divider" />;
}

export type { PickerItem };
