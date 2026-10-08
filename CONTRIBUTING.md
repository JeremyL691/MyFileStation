# 参与 MyFileStation 开发

欢迎用中文或英文提交 Issue 和 Pull Request。新桌面功能以 `src-tauri/` 与 `frontend/` 为主；Python 代码保留作回退与迁移参考。

## 开始之前

先查看[已有问题](https://github.com/JeremyL691/MyFileStation/issues)与[测试矩阵](TESTING.md)。较大的交互调整或平台支持建议先开功能建议，说明具体使用场景。

## 准备开发环境

在 Windows 10 / 11 x64 安装：

- Rust stable、`x86_64-pc-windows-msvc` 目标、rustfmt 与 Clippy（仓库 `rust-toolchain.toml` 已声明）。
- Visual Studio 2022 C++ Build Tools 与 Windows SDK。
- Node.js 22+、pnpm 11、Microsoft WebView2 Runtime。

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

开发版默认使用正式应用标识和用户数据目录。调试前备份重要暂存数据，避免与已安装实例同时运行；如需隔离，请为 Tauri 配置独立应用标识，并为测试进程设置独立 `LOCALAPPDATA` / `APPDATA`。

## 验证改动

```powershell
./scripts/test-windows.ps1 -IncludeStress
```

脚本按顺序执行依赖安装、前端测试 / 类型 / lint / 格式、生产前端构建、Rust fmt / 测试 / Clippy，以及可选存储压力测试。构建桌面包见 [README](README.md#本地开发)。涉及 Python 回退代码时，另运行：

```powershell
python -m pip install -r requirements.txt
python -m pytest -q
```

原生行为需要 Windows 手工验证。涉及拖放、快捷键、剪贴板、窗口定位或自动启动时，请按 [TESTING.md](TESTING.md) 记录相关场景；未执行的场景注明待测。

## 提交 Pull Request

- 说明触发问题的操作、修改后的行为，以及实际执行的验证。
- UI 改动附界面图，使用演示文件，隐藏私人路径与内容。
- 行为修复优先增加能复现问题的回归测试，避免只重复实现细节。
- 同步中英文 README 或相关文档中的受影响说明。
- 提交源码与锁文件；不要提交构建包、数据库、日志、测试运行目录或凭据。

外部文件安全是基本约束：移除暂存条目不能删除外部原文件，文件夹不能被递归清理，拖出采用复制方式。改变这部分行为时请明确说明。

## 仓库导航

参见[架构说明](docs/ARCHITECTURE.md)、[更新记录](CHANGELOG.md)和[最新本地验收报告](QA_REPORT.md)。贡献按仓库 [MIT License](LICENSE) 提供。
