import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, ImportedItems, ShelfItem } from "./types";

export const backend = {
  listItems: () => invoke<ShelfItem[]>("list_items"),
  getSettings: () => invoke<AppSettings>("get_settings"),
  getStartupStatus: () => invoke<boolean>("get_startup_status"),
  updateSettings: (settings: AppSettings) =>
    invoke<AppSettings>("update_settings", { settings }),
  importPaths: (paths: string[]) =>
    invoke<ImportedItems>("import_paths", { paths }),
  pasteClipboard: () => invoke<ImportedItems>("paste_clipboard"),
  setPinned: (itemId: string, pinned: boolean) =>
    invoke<ShelfItem>("set_pinned", { itemId, pinned }),
  removeItems: (itemIds: string[], force = false) =>
    invoke<ShelfItem[]>("remove_items", { itemIds, force }),
  clearUnlocked: () => invoke<ShelfItem[]>("clear_unlocked"),
  copyPaths: (itemIds: string[]) =>
    invoke<number>("copy_paths_to_clipboard", { itemIds }),
  copyPath: (itemId: string) =>
    invoke<void>("copy_path_to_clipboard", { itemId }),
  openItem: (itemId: string) => invoke<void>("open_item", { itemId }),
  revealItem: (itemId: string) => invoke<void>("reveal_item", { itemId }),
  dragOutFinished: (itemIds: string[], succeeded: boolean) =>
    invoke<ShelfItem[]>("handle_drag_out_result", { itemIds, succeeded }),
  showShelf: () => invoke<void>("show_shelf"),
  hideShelf: () => invoke<void>("hide_shelf"),
  showSettings: () => invoke<void>("show_settings"),
  quit: () => invoke<void>("quit_app"),
  windowReady: () => invoke<void>("window_ready"),
  dragIcon: () => invoke<string>("drag_icon_path"),
};
