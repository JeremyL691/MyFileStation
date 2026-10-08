import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
  type ReactNode,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import {
  Check,
  ClipboardPaste,
  File,
  FileImage,
  FileText,
  Folder,
  FolderOpen,
  FolderPlus,
  Grip,
  MoreHorizontal,
  Pin,
  PinOff,
  Search,
  Settings2,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";
import { backend } from "@/lib/backend";
import { messageFor } from "@/lib/errors";
import { filterItems, visibleSelection } from "@/lib/items";
import type { AppSettings, FilterKind, ShelfItem } from "@/lib/types";

type Language = "zh_cn" | "en";
type FilterOption = { id: FilterKind; label: string };

const copy = {
  zh_cn: {
    title: "MyFileStation",
    subtitle: "文件中转站",
    search: "搜索名称或路径…",
    all: "全部",
    file: "文件",
    directory: "文件夹",
    text: "文本",
    image: "图片",
    pinned: "固定",
    paste: "粘贴",
    addFile: "添加文件",
    addFolder: "添加文件夹",
    settings: "设置",
    hide: "隐藏面板",
    clear: "清理未固定",
    emptyTitle: "这里还很空",
    emptyText: "拖入文件，或从剪贴板粘贴内容",
    selected: "已选",
    count: "项内容",
    available: "可用",
    missing: "暂不可用",
    open: "打开",
    reveal: "打开所在位置",
    copyPath: "复制路径",
    copyFiles: "复制文件",
    more: "更多操作",
    unpin: "取消固定",
    pin: "固定",
    remove: "移除",
    clearConfirm: "移除所有未固定条目？固定内容会保留。",
    removeConfirm: "移除所选固定内容？",
    noSelection: "请先选择条目。",
    imported: "已添加",
    duplicates: "个重复项已跳过",
    invalid: "个无效路径已跳过",
    failed: "操作失败，请重试。",
    clipboardBusy: "剪贴板正被其他程序占用。",
    none: "没有可用的条目。",
    drop: "松开以添加到暂存区",
    settingsTitle: "设置",
    close: "关闭",
    dragHint: "拖动到其他应用时始终复制",
  },
  en: {
    title: "MyFileStation",
    subtitle: "File shelf",
    search: "Search name or path…",
    all: "All",
    file: "Files",
    directory: "Folders",
    text: "Text",
    image: "Images",
    pinned: "Pinned",
    paste: "Paste",
    addFile: "Add files",
    addFolder: "Add folders",
    settings: "Settings",
    hide: "Hide shelf",
    clear: "Clear unpinned",
    emptyTitle: "Nothing here yet",
    emptyText: "Drop files here, or paste from your clipboard",
    selected: "selected",
    count: "items",
    available: "Available",
    missing: "Unavailable",
    open: "Open",
    reveal: "Show in folder",
    copyPath: "Copy path",
    copyFiles: "Copy files",
    more: "More actions",
    unpin: "Unpin",
    pin: "Pin",
    remove: "Remove",
    clearConfirm: "Remove all unpinned items? Pinned content will stay.",
    removeConfirm: "Remove the selected pinned items?",
    noSelection: "Select one or more items first.",
    imported: "Added",
    duplicates: "duplicate items skipped",
    invalid: "invalid paths skipped",
    failed: "That action failed. Please try again.",
    clipboardBusy: "The clipboard is busy in another app.",
    none: "No available items.",
    drop: "Drop to add to your shelf",
    settingsTitle: "Settings",
    close: "Close",
    dragHint: "Dragging to another app always copies",
  },
} satisfies Record<Language, Record<string, string>>;

const filterOrder: FilterKind[] = [
  "all",
  "file",
  "directory",
  "text",
  "image",
  "pinned",
];
const iconFor: Record<ShelfItem["kind"], typeof File> = {
  file: File,
  directory: Folder,
  text: FileText,
  image: FileImage,
};

function nativeLanguage(settings?: AppSettings): Language {
  if (settings?.language === "en" || settings?.language === "zh_cn")
    return settings.language;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh_cn" : "en";
}

export default function App() {
  const isSettings =
    new URLSearchParams(window.location.search).get("view") === "settings";
  const [settings, setSettings] = useState<AppSettings>();
  const [items, setItems] = useState<ShelfItem[]>([]);
  const [filter, setFilter] = useState<FilterKind>("all");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [dropActive, setDropActive] = useState(false);
  const [notice, setNotice] = useState("");
  const [menuId, setMenuId] = useState<string | null>(null);
  const [startError, setStartError] = useState("");
  const [settingsReady, setSettingsReady] = useState(false);
  const lastSelected = useRef<string | null>(null);
  const lastEventVersion = useRef(0);
  const refreshVersion = useRef(0);
  const busyRef = useRef(false);
  const pendingDrops = useRef<string[][]>([]);
  const outgoingDrag = useRef({ active: false, selfDrop: false });
  const importPathsRef = useRef<(paths: string[]) => Promise<void>>(
    async () => undefined,
  );
  const language = nativeLanguage(settings);
  const t = copy[language];
  const visibleItems = useMemo(
    () => filterItems(items, filter, query),
    [items, filter, query],
  );
  const visibleSelectedIds = useMemo(
    () => visibleSelection(selected, visibleItems),
    [selected, visibleItems],
  );

  const refresh = useCallback(async () => {
    const version = ++refreshVersion.current;
    const [nextItems, nextSettings] = await Promise.all([
      isSettings ? Promise.resolve([]) : backend.listItems(),
      backend.getSettings(),
    ]);
    if (version !== refreshVersion.current) return;
    setItems(nextItems);
    setSettings(nextSettings);
    setSettingsReady(true);
    setSelected(
      (current) =>
        new Set(
          [...current].filter((id) => nextItems.some((item) => item.id === id)),
        ),
    );
  }, [isSettings]);

  useEffect(() => {
    void refresh().catch((error: unknown) =>
      setStartError(messageFor(error, language)),
    );
    const cleanups: Array<() => void> = [];
    let cancelled = false;
    const registerCleanup = (cleanup: () => void) => {
      if (cancelled) cleanup();
      else cleanups.push(cleanup);
    };
    void (async () => {
      const unlistenChanged = await listen<{ version: number }>(
        "shelf-updated",
        (event) => {
          const version = event.payload.version;
          if (
            Number.isSafeInteger(version) &&
            version > lastEventVersion.current + 1
          ) {
            lastEventVersion.current = version;
            void refresh().catch(() => undefined);
            return;
          }
          if (Number.isSafeInteger(version))
            lastEventVersion.current = Math.max(
              lastEventVersion.current,
              version,
            );
          void refresh().catch(() => undefined);
        },
      );
      registerCleanup(unlistenChanged);
      const unlistenError = await listen<{ message: string }>(
        "startup-error",
        (event) => setStartError(event.payload.message),
      );
      registerCleanup(unlistenError);
      const unlistenDrop = await getCurrentWebview().onDragDropEvent(
        ({ payload }) => {
          if (payload.type === "enter") setDropActive(true);
          if (payload.type === "leave") setDropActive(false);
          if (payload.type === "drop") {
            setDropActive(false);
            if (outgoingDrag.current.active) {
              outgoingDrag.current.selfDrop = true;
              return;
            }
            void importPathsRef.current(payload.paths);
          }
        },
      );
      registerCleanup(unlistenDrop);
    })()
      .catch(() => undefined)
      .finally(() => {
        if (!cancelled)
          void backend
            .windowReady()
            .catch((error: unknown) =>
              setStartError(messageFor(error, language)),
            );
      });
    return () => {
      cancelled = true;
      cleanups.forEach((cleanup) => cleanup());
    };
    // Initial listeners are installed once; refresh reads current data via stable callback.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refresh]);

  useEffect(() => {
    if (!settings) return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const applyTheme = () => {
      const dark =
        settings.theme === "dark" ||
        (settings.theme === "system" && media.matches);
      document.documentElement.classList.toggle("dark", dark);
      document.documentElement.style.colorScheme = dark ? "dark" : "light";
    };
    applyTheme();
    media.addEventListener("change", applyTheme);
    document.documentElement.lang = language === "zh_cn" ? "zh-CN" : "en";
    return () => media.removeEventListener("change", applyTheme);
  }, [settings, language]);

  useEffect(() => {
    if (isSettings) return;
    const handleKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      const target = event.target;
      const typing =
        target instanceof HTMLElement &&
        !!target.closest("input, textarea, select, [contenteditable='true']");
      if (event.key === "Escape") {
        if (selected.size) setSelected(new Set());
        else void backend.hideShelf().catch(reportError);
        return;
      }
      if (typing) return;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "a") {
        event.preventDefault();
        setSelected(new Set(visibleItems.map((item) => item.id)));
      } else if (
        (event.ctrlKey || event.metaKey) &&
        event.key.toLowerCase() === "v"
      ) {
        event.preventDefault();
        void paste();
      } else if (
        (event.ctrlKey || event.metaKey) &&
        event.key.toLowerCase() === "c"
      ) {
        event.preventDefault();
        void copySelected();
      } else if (event.key === "Delete") {
        event.preventDefault();
        void removeSelected(false);
      } else if (event.key === "Enter" || event.key === " ") {
        if (
          target instanceof HTMLElement &&
          target.closest("button, summary, a, [role='menuitem']")
        )
          return;
        if (visibleSelectedIds.length) {
          event.preventDefault();
          void backend.openItem(visibleSelectedIds[0]).catch(reportError);
        }
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  });

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(""), 3400);
    return () => window.clearTimeout(timer);
  }, [notice]);

  function reportError(error: unknown) {
    setNotice(messageFor(error, language));
  }

  async function withBusy(action: () => Promise<void>) {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    try {
      await action();
    } catch (error) {
      reportError(error);
    } finally {
      busyRef.current = false;
      setBusy(false);
      const pending = pendingDrops.current.shift();
      if (pending) void importPathsRef.current(pending);
    }
  }

  async function importPaths(paths: string[]) {
    if (!paths.length) return;
    if (busyRef.current) {
      pendingDrops.current.push(paths);
      return;
    }
    await withBusy(async () => {
      const result = await backend.importPaths(paths);
      await refresh();
      setSelected(new Set(result.items.map((item) => item.id)));
      setNotice(
        `${t.imported} ${result.items.length} · ${result.duplicateCount} ${t.duplicates} · ${result.invalidCount} ${t.invalid}`,
      );
    });
  }

  async function choosePaths(directory: boolean) {
    await withBusy(async () => {
      const chosen = await open({
        multiple: true,
        directory,
        title: directory ? t.addFolder : t.addFile,
      });
      const paths = typeof chosen === "string" ? [chosen] : (chosen ?? []);
      if (!paths.length) return;
      const result = await backend.importPaths(paths);
      await refresh();
      setSelected(new Set(result.items.map((item) => item.id)));
      setNotice(
        `${t.imported} ${result.items.length} · ${result.duplicateCount} ${t.duplicates} · ${result.invalidCount} ${t.invalid}`,
      );
    });
  }

  async function paste() {
    await withBusy(async () => {
      const result = await backend.pasteClipboard();
      if (result.items.length) {
        await refresh();
        setSelected(new Set(result.items.map((item) => item.id)));
        setNotice(`${t.imported} ${result.items.length}`);
      } else if (result.duplicateCount || result.invalidCount) {
        setNotice(
          `${result.duplicateCount} ${t.duplicates} · ${result.invalidCount} ${t.invalid}`,
        );
      } else {
        setNotice(t.none);
      }
    });
  }

  async function copySelected() {
    if (!visibleSelectedIds.length) {
      setNotice(t.noSelection);
      return;
    }
    await withBusy(async () => {
      const count = await backend.copyPaths(visibleSelectedIds);
      setNotice(`${count} ${t.count}`);
    });
  }

  async function removeSelected(force: boolean) {
    const ids = visibleSelectedIds;
    if (!ids.length) return;
    const includesPinned = visibleItems.some(
      (item) => ids.includes(item.id) && item.isPinned,
    );
    const shouldForce =
      includesPinned && force && window.confirm(t.removeConfirm);
    if (includesPinned && force && !shouldForce) return;
    await withBusy(async () => {
      await backend.removeItems(ids, shouldForce);
      setSelected(new Set());
      await refresh();
    });
  }

  async function clearUnpinned() {
    if (!window.confirm(t.clearConfirm)) return;
    await withBusy(async () => {
      await backend.clearUnlocked();
      setSelected(new Set());
      await refresh();
    });
  }

  async function togglePin(item: ShelfItem) {
    await withBusy(async () => {
      await backend.setPinned(item.id, !item.isPinned);
      await refresh();
    });
  }

  useEffect(() => {
    importPathsRef.current = importPaths;
  });

  useEffect(() => {
    lastSelected.current = null;
    setSelected(new Set());
  }, [filter, query]);

  function selectItem(event: MouseEvent | ReactKeyboardEvent, item: ShelfItem) {
    const next = new Set(selected);
    if (event.shiftKey && lastSelected.current) {
      const first = visibleItems.findIndex(
        (entry) => entry.id === lastSelected.current,
      );
      const last = visibleItems.findIndex((entry) => entry.id === item.id);
      if (first >= 0 && last >= 0) {
        for (const entry of visibleItems.slice(
          Math.min(first, last),
          Math.max(first, last) + 1,
        ))
          next.add(entry.id);
      } else {
        next.clear();
        next.add(item.id);
      }
    } else if (event.ctrlKey || event.metaKey) {
      if (next.has(item.id)) next.delete(item.id);
      else next.add(item.id);
    } else {
      next.clear();
      next.add(item.id);
    }
    lastSelected.current = item.id;
    setSelected(next);
  }

  async function dragItem(item: ShelfItem) {
    if (!item.isAvailable) {
      setNotice(t.none);
      return;
    }
    const ids = visibleSelectedIds.includes(item.id)
      ? visibleSelectedIds
      : [item.id];
    const draggedItems = items.filter(
      (entry) => ids.includes(entry.id) && entry.isAvailable,
    );
    const draggedIds = draggedItems.map((entry) => entry.id);
    const paths = draggedItems.map((entry) => entry.path);
    if (!paths.length) {
      setNotice(t.none);
      return;
    }
    await withBusy(async () => {
      const icon = await backend.dragIcon();
      outgoingDrag.current = { active: true, selfDrop: false };
      try {
        await new Promise<void>((resolve, reject) => {
          void startDrag({ item: paths, icon, mode: "copy" }, (result) => {
            void backend
              .dragOutFinished(
                draggedIds,
                result.result === "Dropped" && !outgoingDrag.current.selfDrop,
              )
              .then(refresh)
              .then(resolve, reject);
          }).catch(reject);
        });
      } finally {
        outgoingDrag.current.active = false;
        setDropActive(false);
      }
    });
  }

  function filters(): FilterOption[] {
    return filterOrder.map((id) => ({ id, label: t[id] }));
  }

  if (!settingsReady && !startError)
    return (
      <main className="loading-shell">
        <div className="loading-mark">
          <Grip size={22} />
        </div>
        <span>{t.title}</span>
      </main>
    );
  if (isSettings)
    return (
      <>
        <SettingsPage
          settings={settings}
          busy={busy}
          startError={startError}
          onSettings={setSettings}
          onError={reportError}
        />
        {notice && (
          <div className="toast-message" role="status">
            <span>{notice}</span>
            <button aria-label={t.close} onClick={() => setNotice("")}>
              <X size={14} />
            </button>
          </div>
        )}
      </>
    );

  return (
    <main className="shelf-shell">
      <header className="shelf-header" data-tauri-drag-region>
        <div className="brand-mark">
          <Grip size={19} strokeWidth={2.2} />
        </div>
        <div className="brand-copy">
          <h1>{t.title}</h1>
          <span>{t.subtitle}</span>
        </div>
        <div className="header-actions">
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t.settings}
            title={t.settings}
            onClick={() => void backend.showSettings().catch(reportError)}
          >
            <Settings2 />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t.hide}
            title={t.hide}
            onClick={() => void backend.hideShelf().catch(reportError)}
          >
            <X />
          </Button>
        </div>
      </header>

      {startError && (
        <div className="start-error" role="alert">
          {startError}
        </div>
      )}

      <div className="search-row">
        <div className="search-box">
          <Search size={16} aria-hidden="true" />
          <Input
            aria-label={t.search}
            placeholder={t.search}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setSelected(new Set());
            }}
          />
          {query && (
            <button
              className="clear-search"
              aria-label={t.close}
              onClick={() => setQuery("")}
            >
              <X size={14} />
            </button>
          )}
        </div>
        <Button
          variant="secondary"
          size="icon"
          aria-label={t.paste}
          title={`${t.paste} (Ctrl+V)`}
          disabled={busy}
          onClick={() => void paste()}
        >
          <ClipboardPaste />
        </Button>
      </div>

      <nav className="filter-strip" aria-label={t.subtitle}>
        {filters().map((option) => (
          <button
            key={option.id}
            type="button"
            aria-pressed={filter === option.id}
            className={`filter-chip ${filter === option.id ? "active" : ""}`}
            onClick={() => {
              setFilter(option.id);
              setSelected(new Set());
            }}
          >
            <span>{option.label}</span>
            {option.id === "all" && (
              <span className="filter-count">{items.length}</span>
            )}
          </button>
        ))}
      </nav>

      {selected.size > 0 && (
        <div className="selection-bar">
          <span>
            {visibleSelectedIds.length} {t.selected}
          </span>
          <div className="selection-actions">
            <Button
              variant="ghost"
              size="icon-xs"
              title={t.copyFiles}
              aria-label={t.copyFiles}
              onClick={() => void copySelected()}
            >
              <ClipboardPaste />
            </Button>
            <Button
              variant="ghost"
              size="icon-xs"
              title={t.remove}
              aria-label={t.remove}
              onClick={() => void removeSelected(true)}
            >
              <Trash2 />
            </Button>
            <Button
              variant="ghost"
              size="icon-xs"
              title={t.close}
              aria-label={t.close}
              onClick={() => setSelected(new Set())}
            >
              <X />
            </Button>
          </div>
        </div>
      )}

      <section
        className={`shelf-list ${dropActive ? "drop-active" : ""}`}
        aria-label={t.title}
      >
        {dropActive && (
          <div className="drop-overlay">
            <Upload size={25} />
            <span>{t.drop}</span>
          </div>
        )}
        {!visibleItems.length ? (
          <div className="empty-state">
            <div className="empty-art">
              <FolderOpen size={31} strokeWidth={1.5} />
            </div>
            <strong>{items.length ? t.none : t.emptyTitle}</strong>
            <span>{items.length ? t.search : t.emptyText}</span>
            <div className="empty-actions">
              <Button
                variant="secondary"
                size="sm"
                onClick={() => void choosePaths(false)}
              >
                <File size={15} />
                {t.addFile}
              </Button>
              <Button variant="outline" size="sm" onClick={() => void paste()}>
                <ClipboardPaste size={15} />
                {t.paste}
              </Button>
            </div>
          </div>
        ) : (
          <ScrollArea className="items-scroll">
            <div className="item-list">
              {visibleItems.map((item) => {
                const Icon = iconFor[item.kind];
                const active = selected.has(item.id);
                return (
                  <article
                    key={item.id}
                    className={`item-card ${active ? "selected" : ""} ${!item.isAvailable ? "unavailable" : ""}`}
                    role="option"
                    aria-selected={active}
                    tabIndex={0}
                    onClick={(event) => selectItem(event, item)}
                    onDoubleClick={() => {
                      if (item.isAvailable)
                        void backend.openItem(item.id).catch(reportError);
                    }}
                    onContextMenu={(event) => {
                      event.preventDefault();
                      if (!selected.has(item.id))
                        setSelected(new Set([item.id]));
                      setMenuId(item.id);
                    }}
                    onKeyDown={(event) => {
                      if (event.target !== event.currentTarget) return;
                      if (event.key === "Enter") {
                        event.preventDefault();
                        event.stopPropagation();
                        if (item.isAvailable)
                          void backend.openItem(item.id).catch(reportError);
                      }
                      if (event.key === " ") {
                        event.preventDefault();
                        event.stopPropagation();
                        selectItem(event, item);
                      }
                    }}
                    draggable={false}
                  >
                    <div className="item-leading">
                      {item.thumbnailUrl ? (
                        <img
                          src={item.thumbnailUrl}
                          alt=""
                          className="item-thumbnail"
                        />
                      ) : (
                        <div className={`item-icon kind-${item.kind}`}>
                          <Icon size={18} strokeWidth={1.8} />
                        </div>
                      )}
                    </div>
                    <div className="item-main">
                      <div className="item-title">
                        <span title={item.displayName}>{item.displayName}</span>
                        {item.isPinned && (
                          <Pin
                            size={12}
                            className="pin-mark"
                            aria-label={t.pinned}
                          />
                        )}
                      </div>
                      <div className="item-meta">
                        <Badge variant="secondary">{t[item.kind]}</Badge>
                        <span className="item-path" title={item.path}>
                          {item.path}
                        </span>
                      </div>
                    </div>
                    <button
                      className="item-drag-handle"
                      title={t.dragHint}
                      aria-label={t.dragHint}
                      onPointerDown={(event) => {
                        event.stopPropagation();
                        if (event.button === 0) void dragItem(item);
                      }}
                      onClick={(event) => event.stopPropagation()}
                    >
                      <Grip size={14} />
                    </button>
                    <div className="item-actions">
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        title={item.isPinned ? t.unpin : t.pin}
                        aria-label={item.isPinned ? t.unpin : t.pin}
                        onClick={(event) => {
                          event.stopPropagation();
                          void togglePin(item);
                        }}
                      >
                        {item.isPinned ? <PinOff /> : <Pin />}
                      </Button>
                      <DropdownMenu
                        open={menuId === item.id}
                        onOpenChange={(open) =>
                          setMenuId(open ? item.id : null)
                        }
                      >
                        <DropdownMenuTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon-xs"
                            aria-label={t.more}
                            onClick={(event) => event.stopPropagation()}
                          >
                            <MoreHorizontal size={16} />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent
                          align="end"
                          onClick={(event) => event.stopPropagation()}
                        >
                          <DropdownMenuItem
                            disabled={!item.isAvailable}
                            onSelect={() =>
                              void backend.openItem(item.id).catch(reportError)
                            }
                          >
                            {t.open}
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            disabled={!item.isAvailable}
                            onSelect={() =>
                              void backend
                                .revealItem(item.id)
                                .catch(reportError)
                            }
                          >
                            {t.reveal}
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            disabled={!item.isAvailable}
                            onSelect={() =>
                              void backend
                                .copyPath(item.id)
                                .then(() => setNotice(t.copyPath))
                                .catch(reportError)
                            }
                          >
                            {t.copyPath}
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onSelect={() => void togglePin(item)}
                          >
                            {item.isPinned ? t.unpin : t.pin}
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            variant="destructive"
                            onSelect={() => {
                              setSelected(new Set([item.id]));
                              if (item.isPinned)
                                void removeItemsWithConfirm(item);
                              else
                                void backend
                                  .removeItems([item.id], false)
                                  .then(() => refresh())
                                  .catch(reportError);
                            }}
                          >
                            {t.remove}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                    <span
                      className={`availability ${item.isAvailable ? "" : "missing"}`}
                      title={item.isAvailable ? t.available : t.missing}
                    >
                      {item.isAvailable ? <Check size={12} /> : <X size={12} />}
                    </span>
                  </article>
                );
              })}
            </div>
          </ScrollArea>
        )}
      </section>

      <footer className="shelf-footer">
        <div className="footer-summary">
          <span className="status-dot" />
          {visibleItems.length} {t.count}
          {selected.size > 0 && (
            <span className="footer-selected">
              · {visibleSelectedIds.length} {t.selected}
            </span>
          )}
        </div>
        <div className="footer-actions">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => void choosePaths(false)}
            title={t.addFile}
          >
            <File size={14} />
            {t.addFile}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => void choosePaths(true)}
            title={t.addFolder}
          >
            <FolderPlus size={14} />
            {t.addFolder}
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t.clear}
            title={t.clear}
            onClick={() => void clearUnpinned()}
          >
            <Trash2 />
          </Button>
        </div>
      </footer>
      <div className="drag-hint">{t.dragHint}</div>
      {notice && (
        <div className="toast-message" role="status">
          <span>{notice}</span>
          <button aria-label={t.close} onClick={() => setNotice("")}>
            <X size={14} />
          </button>
        </div>
      )}
    </main>
  );

  async function removeItemsWithConfirm(item: ShelfItem) {
    if (!window.confirm(t.removeConfirm)) return;
    try {
      await backend.removeItems([item.id], true);
      setSelected(new Set());
      await refresh();
    } catch (error) {
      reportError(error);
    }
  }
}

function SettingsPage({
  settings,
  busy,
  startError,
  onSettings,
  onError,
}: {
  settings?: AppSettings;
  busy: boolean;
  startError: string;
  onSettings: (settings: AppSettings) => void;
  onError: (error: unknown) => void;
}) {
  const language: Language =
    settings?.language === "en"
      ? "en"
      : settings?.language === "zh_cn"
        ? "zh_cn"
        : navigator.language.toLowerCase().startsWith("zh")
          ? "zh_cn"
          : "en";
  const t = copy[language];
  const [draft, setDraft] = useState<AppSettings | undefined>(settings);
  const [saving, setSaving] = useState(false);
  const savingRef = useRef(false);
  const [startupStatus, setStartupStatus] = useState<boolean | undefined>();
  useEffect(() => setDraft(settings), [settings]);
  useEffect(() => {
    let active = true;
    void backend
      .getStartupStatus()
      .then((status) => {
        if (active) setStartupStatus(status);
      })
      .catch(() => {
        if (active) setStartupStatus(undefined);
      });
    return () => {
      active = false;
    };
  }, []);
  if (!draft)
    return (
      <main className="settings-shell">
        {startError ? (
          <div className="start-error" role="alert">
            {startError}
          </div>
        ) : (
          <div className="settings-loading">{t.settingsTitle}…</div>
        )}
      </main>
    );

  async function save(next: AppSettings) {
    if (savingRef.current) return;
    savingRef.current = true;
    const previous = settings ?? draft;
    setDraft(next);
    setSaving(true);
    try {
      const saved = await backend.updateSettings(next);
      onSettings(saved);
      setDraft(saved);
      try {
        setStartupStatus(await backend.getStartupStatus());
      } catch (error) {
        setStartupStatus(undefined);
        onError(error);
      }
    } catch (error) {
      setDraft(previous);
      onError(error);
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  }
  const update = (patch: Partial<AppSettings>) =>
    void save({ ...draft, ...patch });
  const close = () => void getCurrentWindow().close().catch(onError);

  return (
    <main className="settings-shell">
      <header className="settings-header" data-tauri-drag-region>
        <div>
          <div className="eyebrow">MYFILESTATION</div>
          <h1>{t.settingsTitle}</h1>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t.close}
          onClick={close}
        >
          <X />
        </Button>
      </header>
      <ScrollArea className="settings-scroll">
        {startError && (
          <div className="start-error" role="alert">
            {startError}
          </div>
        )}
        <fieldset className="settings-content" disabled={saving || busy}>
          <section className="settings-section">
            <div className="settings-section-heading">
              <h2>{language === "zh_cn" ? "外观" : "Appearance"}</h2>
              <p>
                {language === "zh_cn"
                  ? "调整面板位置和视觉偏好。"
                  : "Choose where the shelf opens and how it looks."}
              </p>
            </div>
            <SettingRow
              title={language === "zh_cn" ? "停靠边缘" : "Dock side"}
              description={
                language === "zh_cn"
                  ? "在鼠标所在屏幕的工作区内展开。"
                  : "Open on the work area of the current display."
              }
            >
              <select
                value={draft.dockSide}
                onChange={(event) =>
                  update({
                    dockSide: event.target.value as AppSettings["dockSide"],
                  })
                }
              >
                <option value="right">
                  {language === "zh_cn" ? "右侧" : "Right"}
                </option>
                <option value="left">
                  {language === "zh_cn" ? "左侧" : "Left"}
                </option>
              </select>
            </SettingRow>
            <SettingRow
              title={language === "zh_cn" ? "主题" : "Theme"}
              description={
                language === "zh_cn"
                  ? "使用 Chalk 配色，可跟随系统。"
                  : "Chalk colors with an optional system preference."
              }
            >
              <select
                value={draft.theme}
                onChange={(event) =>
                  update({ theme: event.target.value as AppSettings["theme"] })
                }
              >
                <option value="system">
                  {language === "zh_cn" ? "跟随系统" : "System"}
                </option>
                <option value="light">
                  {language === "zh_cn" ? "浅色" : "Light"}
                </option>
                <option value="dark">
                  {language === "zh_cn" ? "深色" : "Dark"}
                </option>
              </select>
            </SettingRow>
            <SettingRow
              title={language === "zh_cn" ? "语言" : "Language"}
              description={
                language === "zh_cn"
                  ? "默认跟随 Windows 显示语言。"
                  : "Use the Windows display language by default."
              }
            >
              <select
                value={draft.language}
                onChange={(event) =>
                  update({
                    language: event.target.value as AppSettings["language"],
                  })
                }
              >
                <option value="system">
                  {language === "zh_cn" ? "跟随系统" : "System"}
                </option>
                <option value="zh_cn">简体中文</option>
                <option value="en">English</option>
              </select>
            </SettingRow>
          </section>
          <section className="settings-section">
            <div className="settings-section-heading">
              <h2>{language === "zh_cn" ? "行为" : "Behavior"}</h2>
              <p>
                {language === "zh_cn"
                  ? "保护固定内容和外部源文件。"
                  : "Keep pinned content and source files safe."}
              </p>
            </div>
            <SettingRow
              title={
                language === "zh_cn"
                  ? "拖出成功后移除"
                  : "Remove after successful drag"
              }
              description={
                language === "zh_cn"
                  ? "拖动始终复制文件；成功交接后仅移除条目。"
                  : "Dragging always copies. On success, only the shelf entry is removed."
              }
            >
              <input
                type="checkbox"
                checked={draft.removeAfterDragOut}
                onChange={(event) =>
                  update({ removeAfterDragOut: event.target.checked })
                }
              />
            </SettingRow>
            <SettingRow
              title={
                language === "zh_cn"
                  ? "退出时清理临时内容"
                  : "Clear temporary content on exit"
              }
              description={
                language === "zh_cn"
                  ? "固定内容保留，外部文件不会被删除。"
                  : "Pinned items stay; external files are never deleted."
              }
            >
              <input
                type="checkbox"
                checked={draft.cleanupTempOnExit}
                onChange={(event) =>
                  update({ cleanupTempOnExit: event.target.checked })
                }
              />
            </SettingRow>
            <SettingRow
              title={language === "zh_cn" ? "开机启动" : "Start with Windows"}
              description={
                startupStatus === undefined
                  ? language === "zh_cn"
                    ? "系统注册状态：读取中或不可用"
                    : "System status: loading or unavailable"
                  : language === "zh_cn"
                    ? `系统注册状态：${startupStatus ? "已启用" : "未启用"}。登录时在后台启动。`
                    : `System status: ${startupStatus ? "enabled" : "disabled"}. Starts in the background when this user signs in.`
              }
            >
              <input
                type="checkbox"
                checked={draft.autostart}
                onChange={(event) =>
                  update({ autostart: event.target.checked })
                }
              />
            </SettingRow>
          </section>
          <section className="settings-section">
            <div className="settings-section-heading">
              <h2>{language === "zh_cn" ? "快捷键" : "Shortcut"}</h2>
              <p>
                {language === "zh_cn"
                  ? "全局快捷键用于呼出或隐藏面板。"
                  : "A global shortcut toggles the shelf."}
              </p>
            </div>
            <SettingRow
              title={language === "zh_cn" ? "呼出快捷键" : "Toggle shortcut"}
              description={
                language === "zh_cn"
                  ? "示例：Ctrl+Alt+Space"
                  : "Example: Ctrl+Alt+Space"
              }
            >
              <Input
                className="hotkey-input"
                value={draft.hotkey}
                onChange={(event) =>
                  setDraft({ ...draft, hotkey: event.target.value })
                }
                onBlur={() => void save(draft)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.currentTarget.blur();
                  }
                }}
              />
            </SettingRow>
          </section>
          <div className="settings-footnote">
            {language === "zh_cn"
              ? "不记录剪贴板正文 · 不启用遥测 · 设置写入本地数据库"
              : "Clipboard contents are not logged · No telemetry · Settings stay in the local database"}
          </div>
        </fieldset>
      </ScrollArea>
      <div className="settings-bottom">
        <span>
          {saving || busy
            ? language === "zh_cn"
              ? "正在保存…"
              : "Saving…"
            : language === "zh_cn"
              ? "已保存"
              : "Saved"}
        </span>
      </div>
    </main>
  );
}

function SettingRow({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <label className="setting-row">
      <span className="setting-copy">
        <strong>{title}</strong>
        <span>{description}</span>
      </span>
      <span className="setting-control">{children}</span>
    </label>
  );
}
