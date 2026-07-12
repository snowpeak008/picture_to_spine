# FlashToSpine

FlashToSpine 是面向 Windows 的闭源商业 Production Assist 内部候选，用于把用户提供的二次元类人、横版侧视、单主武器角色图片和动作关键帧图片，整理为可修正、可人工审批、可追踪的 Spine 4.2.43 生产资料。

它不是单图一键成片工具，也不生成图片、游戏逻辑或完整 2D 动作游戏。当前状态是 `INTERNAL_CORE_IMPLEMENTATION_CANDIDATE`：尚未完成 GUI 业务全链路、clean-VM 双 runner、代码签名，也没有公开发布授权。

## 当前边界

| 能力 | 当前状态 |
| --- | --- |
| 本地图片、分层、Rig、十动作、人工门与开放格式包 | 内部 Core 已接入；仍需用户逐项审核 |
| 脱敏诊断 JSON | 原生保存对话框与导出已接入；不包含图片、secret、用户名或绝对路径 |
| Spine Professional CLI 4.2.43 | 设置页、宿主和三类异步 job 已接入；真实 4.2.43 运行证据仍为 `NOT_RUN/EXTERNAL` |
| 私有远程 GPU | profile、领域合同、mock、隔离存储和 Credential Manager adapter 已有；真实 HTTPS transport 与远程 job 未接入 |
| AppContainer AI Worker | 未包含，`UNVERIFIED_EXCLUDED` |
| 代码签名与发布 | `NOT_RUN/EXTERNAL`，`releaseAuthorized=false` |

内置 writer 只输出 Rig IR、最小 PSD、透明 PNG、Spine JSON candidate、atlas-input manifest、PromptPack、兼容清单和 checksums。`.atlas`、`.spine`、`.skel` 只能由用户合法持有的 Spine Professional 或适用 Enterprise 4.2.43 生成。

开放包和 CLI 输出都必须选择在整个 `%LOCALAPPDATA%\FlashToSpine` 私有数据根之外；不能借用该根下未枚举的新子目录规避边界。

## 运行入口

- 双击项目根目录的 `FlashToSpineLauncher.exe` 启动已打包的内部候选。
- 打包目录入口为 `dist\FlashToSpine-Core\FlashToSpine.exe`。
- 开发环境可双击 `FlashToSpine-开发入口.cmd`；它只启动已有二进制，不安装依赖或提升权限。

打包物是否对应当前源码，以 `dist\FlashToSpine-Core\package-manifest.json` 的源码绑定和 `npm run test:package` 结果为准。`npm run test:webview-local` 可验证本机窗口/WebView2 启停合同；最终窗口标题只会在内部页面导航成功且 React 根节点与 `.app-shell` 已出现后发布，因此也覆盖启动页 DOM 就绪，但不覆盖 GUI 业务全链路或 clean-VM。存在 exe 不等于已经签名或取得发布授权。

## 架构

桌面端是原生 Win32 + `webview2-com` 直接宿主系统 WebView2 Evergreen Runtime。React、TypeScript 与 PixiJS 由 esbuild 打包并嵌入同一 exe；Rust 是项目 revision、审批、CAS、文件写入和 IPC 的权威端。

`apps/desktop/src-tauri` 只是历史目录名。当前交付不使用 Tauri、Wry 或 Tauri 插件。

## 开发验证

```powershell
npm run bootstrap:check
npm run format:check
npm run lint
npm run typecheck
npm test
npm run test:integration
npm run test:spine
npm run build:ui
npm run build:core
npm run package:core
npm run test:package
```

最终打包后，可选执行本机 GUI 启停探针：

```powershell
npm run test:webview-local
```

所有 `PASS` 只覆盖实际执行的本地检查。GUI 探针固定为 `LOCAL_RUNTIME_ONLY`、`cleanVm=false`，不探测精确 Runtime 版本，也不替代 GUI 业务全链路、真实 Spine Editor/CLI 往返、真实私有远程 GPU、clean-VM、代码签名或组织 Release Gate。

## 文档入口

- [快速开始](docs/user/quick-start-zh-CN.md)
- [人工审批与导出](docs/user/approval-and-export.md)
- [已知限制与项目边界](docs/limits/known-limitations.md)
- [运维手册](docs/maintenance/operations.md)
- [Schema 与迁移维护](docs/maintenance/schema-and-migration.md)
- [当前实施状态审计](evidence/implementation/implementation-status.md)
