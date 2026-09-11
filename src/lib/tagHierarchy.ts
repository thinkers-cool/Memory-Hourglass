import type { TagDto } from "../types";

export function compareTagNames(a: TagDto, b: TagDto): number {
  return a.name.localeCompare(b.name);
}

export function tagChildren(tags: TagDto[], parentId: number): TagDto[] {
  return tags.filter((tag) => tag.parent_id === parentId).sort(compareTagNames);
}

export function tagRoots(tags: TagDto[]): TagDto[] {
  return tags.filter((tag) => tag.parent_id === null).sort(compareTagNames);
}

export function tagDepthFirst(tags: TagDto[]): TagDto[] {
  const ordered: TagDto[] = [];
  const visit = (parentId: number | null) => {
    const children =
      parentId === null ? tagRoots(tags) : tagChildren(tags, parentId);
    for (const child of children) {
      ordered.push(child);
      visit(child.id);
    }
  };
  visit(null);
  return ordered;
}

export function tagPathLabel(tag: TagDto, tags: TagDto[]): string {
  const names: string[] = [];
  let current: TagDto | undefined = tag;
  while (current) {
    names.unshift(current.name);
    current =
      current.parent_id === null
        ? undefined
        : tags.find((entry) => entry.id === current!.parent_id);
  }
  return names.join(" / ");
}

export function collectDescendantTagIds(
  tagId: number,
  tags: TagDto[],
): number[] {
  const ids = new Set<number>([tagId]);
  const visit = (parentId: number) => {
    for (const child of tagChildren(tags, parentId)) {
      ids.add(child.id);
      visit(child.id);
    }
  };
  visit(tagId);
  return [...ids];
}
