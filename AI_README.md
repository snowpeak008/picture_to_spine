# AI 项目导读

> 本文件是所有 AI 助手的通用入口。进入项目后先阅读本文件，再阅读记忆索引。

## AI 会话记忆

本项目使用持久化 AI 记忆系统：

1. 会话开始时读取 `knowledge/ai_memory/INDEX.md`。
2. 若索引提示缓存有效，可直接使用 `project_understanding/`、`code_conventions/`、`decisions/` 中的理解。
3. 若缓存过期，运行 `python tools/memory/check_staleness.py` 查看哪些关键文件变化，再只重读变化部分。
4. 会话结束时，新增 `knowledge/ai_memory/session_history/YYYY-MM-DD-NNN.md`，更新 `knowledge/ai_memory/INDEX.md`。
5. 修改记忆或关键文件后运行 `python tools/memory/update_freshness.py` 更新哈希快照。

## 项目说明

- 项目名称：FlashToSpine。
- 项目用途：Windows 11 x64 闭源商业 Production Assist，把用户提供的二次元类人、横版侧视、单主武器角色图片及动作关键帧整理为可修正、可人工审批、可追踪的 Spine 4.2.43 生产候选；它不是单图一键成片器或完整游戏制作器。
- 当前状态：`INTERNAL_CORE_IMPLEMENTATION_CANDIDATE`；GUI 全业务链、真实 Spine 往返、真实私有 GPU、clean-VM、签名和发布授权仍未完成。
- 主要入口：根目录 `FlashToSpineLauncher.exe`；开发入口 `FlashToSpine-开发入口.cmd`；包入口 `dist/FlashToSpine-Core/FlashToSpine.exe`。
- 构建/测试命令：以根目录 `package.json` 为准；常用全链为 `npm run format:check`、`npm run lint`、`npm run typecheck`、`npm test`、`npm run test:integration`、`npm run test:spine`、`npm run package:core`、`npm run test:package`，打包后可执行 `npm run test:webview-local`。
- 重要目录：`apps/desktop-ui`（React/TypeScript/PixiJS）、`apps/desktop/src-tauri`（历史目录名，实际是原生 Win32/WebView2 Rust 宿主）、`crates`（Domain/Application/Adapters）、`docs`、`plan/devplan`、`evidence`、`knowledge/ai_memory`。
- 权威长期事实：`PROJECT_MEMORY.md`；实施裁决和证据：`evidence/implementation/implementation-status.md`、`evidence/implementation/final-validation.json`。

## 开发规则

- 代码风格：Rust 2024，提交前必须通过 `cargo fmt --all -- --check`；TypeScript 保持严格类型检查。Domain 不依赖外层，依赖方向固定为 Domain ← Application ← Adapter/Delivery。
- 测试要求：改动对应层必须补充或更新测试；跨层改动同时核对 TypeScript、IPC、Rust DTO/命令和持久化合同。`PASS` 只能描述实际覆盖范围，不能把 mock/static/local 结果升级为真实外部能力或 clean-VM。
- 数据规则：Rust 是 revision、审批、CAS、原生文件对话框、持久化和 IPC 的权威端；审批绑定当前 revision/hash；动画内部使用整数 tick（`timeBase=1/30000`）。
- 启动规则：WebView2 `NavigateToString` 只允许 `about:blank` 与本次嵌入 HTML 的精确 data URI；最终标题只能在导航成功且 React 根节点/`.app-shell` DOM 就绪后发布。
- 依赖规则：发布依赖只允许审计通过的 MIT、Apache-2.0、BSD 等宽松许可；新增依赖必须更新许可清单并运行许可检查。
- 禁止提交：密钥、token、Cookie、Spine 激活信息、私有证书、DPAPI 明文、本地绝对路径配置、角色原图/CAS 内容、临时缓存和包含敏感信息的日志。
- 本地配置：项目、CAS、staging、配置、密钥密文和 WebView2 数据位于 `%LOCALAPPDATA%\FlashToSpine`；不要手改 head/anchor/sidecar，不要自动删除该数据根。
- 文件操作：保留用户已有改动；源码编辑使用可审查补丁；不得用旧 evidence 或旧 EXE 哈希证明新构建。
- 发布规则：当前 `releaseAuthorized=false`。构建成功不等于签名、兼容性、Production Ready 或公开发布授权。
