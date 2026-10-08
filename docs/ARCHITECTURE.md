# 架构与仓库导航

MyFileStation 0.3.0 将 Windows 原生能力放在 Rust / Tauri 2 中，React / TypeScript 负责主面板与设置页。数据保存在当前用户本地，不需要服务端。

## 目录

| 路径 | 职责 |
| --- | --- |
| `frontend/src/App.tsx` | 主面板、选择与筛选、原生事件、拖放与设置交互 |
| `frontend/src/lib/` | Tauri 命令入口、类型、筛选与错误提示 |
| `frontend/src/components/ui/` | 基于 Radix 的界面组件 |
| `src-tauri/src/lib.rs` | 命令、插件、应用生命周期、快捷键、剪贴板与自动启动 |
| `src-tauri/src/store.rs` | SQLite、路径去重、生成内容、延迟清理与恢复 |
| `src-tauri/src/windows.rs` | Windows 窗口定位、边缘检测与文件剪贴板 |
| `src-tauri/src/tray.rs` | 系统托盘菜单 |
| `scripts/` | Windows 构建、验收、便携启动与打包 |
| `.github/` | Windows CI、问题表单、PR 模板 |
| `src/myfilestation/` / `tests/` | 旧版 Python 应用与回退测试 |

## 内容流转

```mermaid
flowchart LR
    A[外部文件 / 文件夹] -->|仅保存路径引用| S[SQLite 暂存记录]
    B[剪贴板文字 / 图片] --> G[应用生成的本地文件]
    G --> S
    S --> UI[React 暂存面板]
    UI -->|原生复制拖出 / 文件剪贴板| T[支持接收文件的目标应用]
    UI -->|移除条目| R[更新记录]
    R -->|仅生成文件| C[延迟回收队列]
```

外部文件由其原位置管理；应用只删除自身拥有且符合生成文件规则的内容。回收前检查活跃条目及剪贴板引用，避免删除仍在使用的生成文件。

## 本地数据

`%LOCALAPPDATA%\MyFileStation\v2` 包含：

- `station.db`：条目、设置与待删除记录。
- `artifacts/`：粘贴生成的文字与图片。
- `thumbnails/`：图片缩略图。
- `logs/`：应用诊断日志。

首次运行读取可识别的旧版设置，保留旧设置文件；不迁移旧版暂存条目。修改数据库结构时需要版本检查与迁移路径，不能遇到未知版本直接覆盖。

## 验证边界

Vitest 验证前端状态与交互，Rust 测试验证持久化、安全回收与平台边界计算。Windows 原生拖放及跨应用行为需要实际桌面验证。详细范围见 [测试矩阵](../TESTING.md) 与 [验收报告](../QA_REPORT.md)。
