type Language = "zh_cn" | "en";

const messages: Record<string, [string, string]> = {
  clipboardBusy: [
    "剪贴板正被其他程序占用，请稍后重试。",
    "The clipboard is busy in another app. Try again shortly.",
  ],
  noAvailableItems: ["没有可用的条目。", "No available items."],
  clipboardTextEmpty: ["剪贴板中没有文本。", "The clipboard contains no text."],
  clipboardTextTooLarge: [
    "剪贴板文本超过 32 MB，无法添加。",
    "Clipboard text exceeds the 32 MB limit.",
  ],
  databaseCorrupt: [
    "本地数据库已损坏，原文件已保留。请备份数据后恢复数据库。",
    "The local database is corrupt and has been preserved. Back up the data before restoring the database.",
  ],
  databaseFromNewerVersion: [
    "数据库来自较新版本，请升级应用后打开。",
    "This database requires a newer application version. Upgrade the app to open it.",
  ],
  storageUnavailable: [
    "无法打开本地数据库。请检查数据目录权限并重启应用。",
    "Cannot open the local database. Check data directory permissions and restart the app.",
  ],
  storageOperationFailed: [
    "本地数据库操作失败，请检查磁盘空间和数据目录权限。",
    "The database operation failed. Check disk space and data directory permissions.",
  ],
  settingsCouldNotRead: [
    "无法读取已保存的设置，请备份数据后检查数据库。",
    "Saved settings could not be read. Back up the data and check the database.",
  ],
  settingsCouldNotSave: [
    "设置保存失败，已恢复之前的值。",
    "Settings could not be saved. The previous value has been restored.",
  ],
  settingsUnavailable: [
    "设置暂不可用，请重启应用。",
    "Settings are unavailable. Restart the app.",
  ],
  shortcutConflict: [
    "此快捷键已被占用，请选择其他组合。原快捷键仍可使用。",
    "This shortcut is already in use. Choose another combination. Your previous shortcut remains active.",
  ],
  shortcutInvalid: [
    "快捷键无效，请使用 Ctrl+Alt+Space 等组合。",
    "Invalid shortcut. Use a combination such as Ctrl+Alt+Space.",
  ],
  shortcutUpdateFailed: [
    "快捷键更新失败，请重试。",
    "The shortcut could not be updated. Please try again.",
  ],
  autostartFailed: [
    "无法读取或更新开机启动状态，请检查当前用户权限。",
    "Windows startup status could not be read or updated. Check your user permissions.",
  ],
  fileUnavailable: [
    "源文件暂不可用，请检查文件位置或网络连接。",
    "The source file is unavailable. Check its location or your network connection.",
  ],
  itemNotFound: [
    "条目已被移除，请刷新后重试。",
    "This item has been removed. Refresh and try again.",
  ],
  pathInvalid: [
    "文件路径无效或不在允许的存储位置。",
    "The file path is invalid or outside managed storage.",
  ],
  imageDimensionsInvalid: [
    "图片尺寸无效或超过限制。",
    "The image dimensions are invalid or exceed the limit.",
  ],
  imageTooLarge: [
    "图片超过 4000 万像素限制，请缩小后重试。",
    "The image exceeds the 40 million pixel limit. Resize it and try again.",
  ],
  imageCouldNotBeSaved: [
    "图片保存失败，请检查磁盘空间和目录权限。",
    "The image could not be saved. Check disk space and directory permissions.",
  ],
  dragCouldNotStart: [
    "无法启动拖放，请检查应用资源是否完整。",
    "Dragging could not start. Check that application resources are intact.",
  ],
  openFailed: [
    "无法打开文件，请检查默认打开程序。",
    "The file could not be opened. Check its default application.",
  ],
  revealFailed: [
    "无法打开文件所在位置，请重试。",
    "The file location could not be opened. Please try again.",
  ],
  fileOperationFailed: [
    "文件操作失败，请检查路径、磁盘空间和权限。",
    "The file operation failed. Check the path, disk space, and permissions.",
  ],
  windowUnavailable: [
    "应用窗口不可用，请重启应用。",
    "The app window is unavailable. Restart the app.",
  ],
  windowOperationFailed: [
    "窗口操作失败，请重试。",
    "The window operation failed. Please try again.",
  ],
  operationFailed: [
    "操作失败，请重试。",
    "That action failed. Please try again.",
  ],
};

export function messageFor(error: unknown, language: Language): string {
  const index = language === "zh_cn" ? 0 : 1;
  if (typeof error === "object" && error && "messageKey" in error) {
    const key = String(error.messageKey).replace(/^error\./, "");
    if (messages[key]) return messages[key][index];
  }
  return messages.operationFailed[index];
}
