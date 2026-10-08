export type ItemKind = "file" | "directory" | "text" | "image";
export type ItemSource = "external" | "generated";

export interface ShelfItem {
  id: string;
  kind: ItemKind;
  source: ItemSource;
  path: string;
  displayName: string;
  isPinned: boolean;
  addedAt: number;
  isAvailable: boolean;
  thumbnailUrl: string | null;
}

export interface AppSettings {
  dockSide: "left" | "right";
  removeAfterDragOut: boolean;
  cleanupTempOnExit: boolean;
  autostart: boolean;
  hotkey: string;
  theme: "light" | "dark" | "system";
  language: "zh_cn" | "en" | "system";
}

export interface ImportedItems {
  items: ShelfItem[];
  duplicateCount: number;
  invalidCount: number;
}

export type FilterKind =
  "all" | "file" | "directory" | "text" | "image" | "pinned";
