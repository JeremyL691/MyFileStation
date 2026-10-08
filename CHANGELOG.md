# 更新记录

## 0.3.0 — 未发布

当前为发布候选阶段，完整验收状态见 [QA_REPORT.md](QA_REPORT.md)。

### 新版桌面应用

- 使用 Rust、Tauri 2、React 与 TypeScript 重建桌面端，保留 Python / PySide6 回退实现。
- 支持名称与路径搜索、类型筛选、多选、固定条目和 SQLite 持久化。
- 提供中英文、浅色 / 深色 / 系统主题，以及独立设置窗口。
- 提供当前用户 NSIS 安装包、便携 ZIP 与 SHA-256 校验文件。

### 验收修复

- 修复 Windows 设置窗口白屏、跨屏缩放裁切、长路径挤出操作按钮和菜单裁切。
- 修复并发导入丢失、拖放取消与拖回自身时的错误清理，以及键盘事件重复处理。
- 分离复制文件与复制路径，修复原生文件剪贴板拥有者设置。
- 加强设置保存与快捷键回滚、启动错误提示、第二实例处理和退出流程。
- 加强路径规范化、数据库损坏保护、生成文件写入失败恢复与孤立缩略图回收。
- 增加回归测试、存储压力测试、统一验收脚本与 Windows CI。

### 仓库展示

- 增加中英文首页、工作流横幅、使用与开发指南。
- 增加问题反馈表单、功能建议表单、PR 模板和架构说明。

## v0.2.0 — 已发布

Python / PySide6 版本。下载与该版本说明见 [GitHub Release](https://github.com/JeremyL691/MyFileStation/releases/tag/v0.2.0)。

## v0.1.0 — 已发布

早期 Windows 版本。下载与该版本说明见 [GitHub Release](https://github.com/JeremyL691/MyFileStation/releases/tag/v0.1.0)。
