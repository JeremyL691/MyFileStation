import { describe, expect, it } from "vitest";
import { filterItems, visibleSelection } from "./items";
import type { ShelfItem } from "./types";

const items: ShelfItem[] = [
  {
    id: "a",
    kind: "file",
    source: "external",
    path: "C:\\work\\Report.docx",
    displayName: "Report.docx",
    isPinned: true,
    addedAt: 3,
    isAvailable: true,
    thumbnailUrl: null,
  },
  {
    id: "b",
    kind: "directory",
    source: "external",
    path: "C:\\work\\images",
    displayName: "images",
    isPinned: false,
    addedAt: 2,
    isAvailable: true,
    thumbnailUrl: null,
  },
  {
    id: "c",
    kind: "text",
    source: "generated",
    path: "C:\\local\\clip.txt",
    displayName: "Clipboard text",
    isPinned: false,
    addedAt: 1,
    isAvailable: true,
    thumbnailUrl: null,
  },
];

describe("shelf filtering", () => {
  it("searches names and paths without case sensitivity", () => {
    expect(filterItems(items, "all", "REPORT").map((item) => item.id)).toEqual([
      "a",
    ]);
    expect(filterItems(items, "all", "LOCAL").map((item) => item.id)).toEqual([
      "c",
    ]);
  });

  it("keeps folders in the files filter and isolates pinned items", () => {
    expect(filterItems(items, "file", "").map((item) => item.id)).toEqual([
      "a",
      "b",
    ]);
    expect(filterItems(items, "pinned", "").map((item) => item.id)).toEqual([
      "a",
    ]);
  });

  it("limits batch selection to currently visible entries", () => {
    expect(
      visibleSelection(new Set(["a", "c"]), filterItems(items, "file", "")),
    ).toEqual(["a"]);
  });
});
