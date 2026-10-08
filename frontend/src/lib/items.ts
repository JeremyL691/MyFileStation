import type { FilterKind, ShelfItem } from "./types";

export function filterItems(
  items: ShelfItem[],
  filter: FilterKind,
  query: string,
) {
  const needle = query.trim().toLocaleLowerCase();
  return items.filter((item) => {
    const matchesFilter =
      filter === "all" ||
      (filter === "pinned"
        ? item.isPinned
        : item.kind === filter ||
          (filter === "file" && item.kind === "directory"));
    const matchesQuery =
      !needle ||
      item.displayName.toLocaleLowerCase().includes(needle) ||
      item.path.toLocaleLowerCase().includes(needle);
    return matchesFilter && matchesQuery;
  });
}

export function visibleSelection(
  selectedIds: Set<string>,
  visibleItems: ShelfItem[],
) {
  return visibleItems
    .filter((item) => selectedIds.has(item.id))
    .map((item) => item.id);
}
