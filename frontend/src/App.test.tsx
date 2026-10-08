import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { AppSettings, ShelfItem } from "@/lib/types";

const mocks = vi.hoisted(() => ({
  listItems: vi.fn(),
  getSettings: vi.fn(),
  getStartupStatus: vi.fn(),
  updateSettings: vi.fn(),
  removeItems: vi.fn(),
  windowReady: vi.fn(),
  hideShelf: vi.fn(),
  openItem: vi.fn(),
  showSettings: vi.fn(),
  getCurrentWebview: vi.fn(),
  listen: vi.fn(),
  openDialog: vi.fn(),
  dragIcon: vi.fn(),
  dragOutFinished: vi.fn(),
  startDrag: vi.fn(),
  copyPath: vi.fn(),
  importPaths: vi.fn(),
}));

vi.mock("@/lib/backend", () => ({
  backend: {
    listItems: mocks.listItems,
    getSettings: mocks.getSettings,
    getStartupStatus: mocks.getStartupStatus,
    updateSettings: mocks.updateSettings,
    removeItems: mocks.removeItems,
    windowReady: mocks.windowReady,
    hideShelf: mocks.hideShelf,
    openItem: mocks.openItem,
    showSettings: mocks.showSettings,
    dragIcon: mocks.dragIcon,
    dragOutFinished: mocks.dragOutFinished,
    copyPath: mocks.copyPath,
    importPaths: mocks.importPaths,
  },
}));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: mocks.getCurrentWebview,
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ close: vi.fn() }),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.openDialog }));
vi.mock("@crabnebula/tauri-plugin-drag", () => ({
  startDrag: mocks.startDrag,
}));

const baseSettings: AppSettings = {
  dockSide: "right",
  removeAfterDragOut: true,
  cleanupTempOnExit: true,
  autostart: false,
  hotkey: "Ctrl+Alt+Space",
  theme: "light",
  language: "en",
};
const fixture: ShelfItem[] = [
  {
    id: "one",
    kind: "file",
    source: "external",
    path: "C:\\work\\alpha.txt",
    displayName: "alpha.txt",
    isPinned: false,
    addedAt: 2,
    isAvailable: true,
    thumbnailUrl: null,
  },
  {
    id: "two",
    kind: "text",
    source: "generated",
    path: "C:\\local\\note.txt",
    displayName: "Clipboard note",
    isPinned: true,
    addedAt: 1,
    isAvailable: true,
    thumbnailUrl: null,
  },
];

afterEach(cleanup);

describe("shelf interactions", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    window.history.replaceState({}, "", "/");
    mocks.listItems.mockResolvedValue(fixture);
    mocks.getSettings.mockResolvedValue(baseSettings);
    mocks.getStartupStatus.mockResolvedValue(false);
    mocks.updateSettings.mockImplementation(
      async (settings: AppSettings) => settings,
    );
    mocks.removeItems.mockResolvedValue([]);
    mocks.windowReady.mockResolvedValue(undefined);
    mocks.hideShelf.mockResolvedValue(undefined);
    mocks.openItem.mockResolvedValue(undefined);
    mocks.showSettings.mockResolvedValue(undefined);
    mocks.getCurrentWebview.mockReturnValue({
      onDragDropEvent: vi.fn().mockResolvedValue(vi.fn()),
    });
    mocks.listen.mockResolvedValue(vi.fn());
    mocks.openDialog.mockResolvedValue(null);
    mocks.dragIcon.mockResolvedValue("C:\\app\\drag-icon.png");
    mocks.dragOutFinished.mockResolvedValue([]);
    mocks.copyPath.mockResolvedValue(undefined);
    mocks.importPaths.mockResolvedValue({
      items: [],
      duplicateCount: 0,
      invalidCount: 0,
    });
    mocks.startDrag.mockImplementation(async (_options, callback) =>
      callback({ result: "Dropped" }),
    );
  });

  it("opens a multi-select folder picker from the shelf footer", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(
      await screen.findByRole("button", { name: "Add folders" }),
    );
    expect(mocks.openDialog).toHaveBeenCalledWith(
      expect.objectContaining({ directory: true, multiple: true }),
    );
  });

  it("filters names and paths, and clears selection when the filter changes", async () => {
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByText("alpha.txt")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    expect(screen.getByText("2 selected")).toBeInTheDocument();
    await user.type(
      screen.getByRole("textbox", { name: "Search name or path…" }),
      "note",
    );
    expect(screen.getByText("Clipboard note")).toBeInTheDocument();
    expect(screen.queryByText("2 selected")).not.toBeInTheDocument();
    expect(screen.queryByText("alpha.txt")).not.toBeInTheDocument();
  });

  it("removes the visible selection on Delete while leaving pinned rows alone", async () => {
    render(<App />);
    const alpha = await screen.findByRole("option", { name: /alpha\.txt/ });
    fireEvent.click(alpha);
    fireEvent.keyDown(window, { key: "Delete" });
    await waitFor(() =>
      expect(mocks.removeItems).toHaveBeenCalledWith(["one"], false),
    );
  });

  it("rolls back a setting change when persistence fails", async () => {
    window.history.replaceState({}, "", "/?view=settings");
    mocks.updateSettings.mockRejectedValue({
      code: "STORAGE_ERROR",
      messageKey: "error.settingsCouldNotSave",
    });
    const user = userEvent.setup();
    render(<App />);
    expect(
      await screen.findByText(/System status: disabled/),
    ).toBeInTheDocument();
    const themeSelect = await screen.findByDisplayValue("Light");
    await user.selectOptions(themeSelect, "dark");
    await waitFor(() => expect(mocks.updateSettings).toHaveBeenCalled());
    expect(themeSelect).toHaveValue("light");
    expect(screen.getByRole("status")).toHaveTextContent(/could not|failed/i);
  });

  it("opens a focused row only once on Enter", async () => {
    render(<App />);
    const row = await screen.findByRole("option", { name: /alpha\.txt/ });
    fireEvent.click(row);
    fireEvent.keyDown(row, { key: "Enter" });
    expect(mocks.openItem).toHaveBeenCalledTimes(1);
  });

  it("selects a focused row on Space without opening it", async () => {
    render(<App />);
    const row = await screen.findByRole("option", { name: /alpha\.txt/ });
    fireEvent.click(row);
    fireEvent.keyDown(row, { key: " " });
    expect(row).toHaveAttribute("aria-selected", "true");
    expect(mocks.openItem).not.toHaveBeenCalled();
  });

  it("does not open a selected file when Enter is pressed on a toolbar button", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("option", { name: /alpha\.txt/ }));
    fireEvent.keyDown(screen.getByRole("button", { name: "Settings" }), {
      key: "Enter",
    });
    expect(mocks.openItem).not.toHaveBeenCalled();
  });

  it("only removes available files actually included in a successful drag", async () => {
    mocks.listItems.mockResolvedValue([
      fixture[0],
      { ...fixture[1], isAvailable: false },
    ]);
    render(<App />);
    await screen.findByText("alpha.txt");
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    fireEvent.pointerDown(
      screen.getAllByRole("button", {
        name: "Dragging to another app always copies",
      })[0],
      { button: 0 },
    );
    await waitFor(() =>
      expect(mocks.dragOutFinished).toHaveBeenCalledWith(["one"], true),
    );
  });

  it("waits for a drag callback that arrives after the native invocation resolves", async () => {
    let finishDrag: (result: { result: string }) => void = () => undefined;
    mocks.startDrag.mockImplementation(async (_options, callback) => {
      finishDrag = callback;
    });
    render(<App />);
    await screen.findByText("alpha.txt");
    fireEvent.pointerDown(
      screen.getAllByRole("button", {
        name: "Dragging to another app always copies",
      })[0],
      { button: 0 },
    );
    await waitFor(() => expect(mocks.startDrag).toHaveBeenCalled());
    expect(mocks.dragOutFinished).not.toHaveBeenCalled();
    finishDrag({ result: "Dropped" });
    await waitFor(() =>
      expect(mocks.dragOutFinished).toHaveBeenCalledWith(["one"], true),
    );
  });

  it("disables settings controls while a write is pending", async () => {
    window.history.replaceState({}, "", "/?view=settings");
    mocks.updateSettings.mockReturnValue(new Promise(() => undefined));
    const user = userEvent.setup();
    render(<App />);
    const themeSelect = await screen.findByDisplayValue("Light");
    await user.selectOptions(themeSelect, "dark");
    expect(themeSelect).toBeDisabled();
    expect(screen.getByDisplayValue("English")).toBeDisabled();
  });

  it("keeps a saved setting when reading the startup registry subsequently fails", async () => {
    window.history.replaceState({}, "", "/?view=settings");
    mocks.getStartupStatus
      .mockResolvedValueOnce(false)
      .mockRejectedValueOnce(new Error("Registry unreadable"));
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText(/System status: disabled/);
    const themeSelect = screen.getByDisplayValue("Light");
    await user.selectOptions(themeSelect, "dark");
    await waitFor(() =>
      expect(mocks.getStartupStatus).toHaveBeenCalledTimes(2),
    );
    expect(themeSelect).toHaveValue("dark");
  });

  it("uses text clipboard copying for the Copy path context action", async () => {
    const user = userEvent.setup();
    render(<App />);
    fireEvent.contextMenu(
      await screen.findByRole("option", { name: /alpha\.txt/ }),
    );
    await user.click(
      await screen.findByRole("menuitem", { name: "Copy path" }),
    );
    expect(mocks.copyPath).toHaveBeenCalledWith("one");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("reports a canceled drag without removing its entries", async () => {
    mocks.startDrag.mockImplementation(async (_options, callback) =>
      callback({ result: "Cancel" }),
    );
    render(<App />);
    await screen.findByText("alpha.txt");
    fireEvent.pointerDown(
      screen.getAllByRole("button", {
        name: "Dragging to another app always copies",
      })[0],
      { button: 0 },
    );
    await waitFor(() =>
      expect(mocks.dragOutFinished).toHaveBeenCalledWith(["one"], false),
    );
    expect(mocks.removeItems).not.toHaveBeenCalled();
  });

  it("shows an actionable startup error after the webview is ready", async () => {
    mocks.windowReady.mockRejectedValue({
      code: "DATABASE_CORRUPT",
      messageKey: "error.databaseCorrupt",
    });
    render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      /database is corrupt.*preserved/i,
    );
  });

  it("does not overwrite a newer refresh with an older response", async () => {
    let firstResponse: (items: ShelfItem[]) => void = () => undefined;
    let changed: (event: { payload: { version: number } }) => void = () =>
      undefined;
    mocks.listItems.mockReturnValueOnce(
      new Promise<ShelfItem[]>((resolve) => {
        firstResponse = resolve;
      }),
    );
    mocks.listen.mockImplementation(async (name, callback) => {
      if (name === "shelf-updated") changed = callback;
      return vi.fn();
    });
    render(<App />);
    await waitFor(() => expect(mocks.listen).toHaveBeenCalled());
    act(() => changed({ payload: { version: 1 } }));
    await screen.findByText("alpha.txt");
    await act(async () => firstResponse([]));
    expect(screen.getByText("alpha.txt")).toBeInTheDocument();
  });

  it("queues native drops without overlapping imports or losing the later drop", async () => {
    let drop: (event: {
      payload: { type: string; paths: string[] };
    }) => void = () => undefined;
    let finishFirst: (result: {
      items: ShelfItem[];
      duplicateCount: number;
      invalidCount: number;
    }) => void = () => undefined;
    mocks.getCurrentWebview.mockReturnValue({
      onDragDropEvent: vi.fn(async (callback) => {
        drop = callback;
        return vi.fn();
      }),
    });
    mocks.importPaths.mockReturnValueOnce(
      new Promise((resolve) => {
        finishFirst = resolve;
      }),
    );
    render(<App />);
    await screen.findByText("alpha.txt");
    await waitFor(() => expect(mocks.windowReady).toHaveBeenCalled());
    act(() => drop({ payload: { type: "drop", paths: ["C:\\first.txt"] } }));
    act(() => drop({ payload: { type: "drop", paths: ["C:\\second.txt"] } }));
    expect(mocks.importPaths).toHaveBeenCalledTimes(1);
    await act(async () =>
      finishFirst({ items: [], duplicateCount: 0, invalidCount: 0 }),
    );
    await waitFor(() =>
      expect(mocks.importPaths).toHaveBeenNthCalledWith(2, ["C:\\second.txt"]),
    );
  });

  it("releases earlier listeners if a later registration fails", async () => {
    const unlisten = vi.fn();
    mocks.listen
      .mockResolvedValueOnce(unlisten)
      .mockRejectedValueOnce(new Error("Registration failed"));
    const view = render(<App />);
    await waitFor(() => expect(mocks.windowReady).toHaveBeenCalled());
    view.unmount();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("retains entries when files are dragged back onto the same shelf", async () => {
    let drop: (event: {
      payload: { type: string; paths: string[] };
    }) => void = () => undefined;
    mocks.getCurrentWebview.mockReturnValue({
      onDragDropEvent: vi.fn(async (callback) => {
        drop = callback;
        return vi.fn();
      }),
    });
    mocks.startDrag.mockImplementation(async (_options, callback) => {
      drop({ payload: { type: "drop", paths: [fixture[0].path] } });
      callback({ result: "Dropped" });
    });
    render(<App />);
    await screen.findByText("alpha.txt");
    await waitFor(() => expect(mocks.windowReady).toHaveBeenCalled());
    fireEvent.pointerDown(
      screen.getAllByRole("button", {
        name: "Dragging to another app always copies",
      })[0],
      { button: 0 },
    );
    await waitFor(() =>
      expect(mocks.dragOutFinished).toHaveBeenCalledWith(["one"], false),
    );
    expect(mocks.importPaths).not.toHaveBeenCalled();
  });

  it("updates a system theme when the operating system preference changes", async () => {
    const media = new EventTarget() as EventTarget & { matches: boolean };
    media.matches = false;
    vi.spyOn(window, "matchMedia").mockReturnValue(media as MediaQueryList);
    mocks.getSettings.mockResolvedValue({ ...baseSettings, theme: "system" });
    render(<App />);
    await screen.findByText("alpha.txt");
    expect(document.documentElement).not.toHaveClass("dark");
    act(() => {
      media.matches = true;
      media.dispatchEvent(new Event("change"));
    });
    expect(document.documentElement).toHaveClass("dark");
    expect(document.documentElement.style.colorScheme).toBe("dark");
  });
});
