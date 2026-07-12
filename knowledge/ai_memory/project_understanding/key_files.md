# 关键文件清单

> 第一次接入项目后填写。这里只记录理解项目所需的关键文件，不要把所有源码都列进来。

## 入口与配置

| 文件 | 作用 | 读取状态 |
|---|---|---|
| `README.md` | 项目用户说明 | 已读 |
| `AI_README.md` | AI 协作入口 | 已读 |
| `AGENTS.md` | Codex 入口 | 已读 |
| `PROJECT_MEMORY.md` | 权威项目长期事实和恢复顺序 | 已读 |
| `package.json` | Node/UI/构建/验证命令 | 已读 |
| `Cargo.toml` / `Cargo.lock` | Rust workspace、固定依赖和工具链合同 | 已读 |

## 核心代码

| 文件 | 作用 | 读取状态 |
|---|---|---|
| `apps/desktop/src-tauri/src/main.rs` | Win32/WebView2 启动、安全导航和 DOM 就绪门禁 | 已读 |
| `apps/desktop/src-tauri/src/ipc_host.rs` | UI→Rust IPC 权威宿主 | 按需读取 |
| `apps/desktop-ui/src/app/AppShell.tsx` | UI 工作台总入口 | 按需读取 |
| `apps/desktop-ui/src/state/projectStore.ts` | 前端项目状态与 IPC 调用编排 | 按需读取 |
| `crates/domain/src/lib.rs` | 领域模块入口 | 按需读取 |
| `crates/application/src/lib.rs` | 应用用例入口 | 按需读取 |
| `crates/adapters/src/lib.rs` | 适配器入口 | 按需读取 |
| `schemas/src/ipc.schema.json` | IPC 合同源文件 | 按需读取 |

## 测试与验证

| 文件/命令 | 作用 | 读取状态 |
|---|---|---|
| `evidence/implementation/implementation-status.md` | 实施覆盖与未关闭边界 | 已读 |
| `evidence/implementation/final-validation.json` | 当前包、检查与哈希绑定 | 已读 |
| `docs/limits/known-limitations.md` | 产品不能做什么 | 已读 |
| `docs/maintenance/operations.md` | 构建、运行、存储和外部系统运维规则 | 已读 |
| `tests/unit/webview2-startup-probe-contract.test.mjs` | 启动导航/DOM 防回归合同 | 已读 |
| `npm test` / `npm run test:integration` | Node/UI + Rust workspace/跨层验证 | 最近通过 |
| `npm run test:package` / `npm run test:webview-local` | 包绑定和本机 GUI 启动验证 | 最近通过 |

## freshness 维护

把需要参与哈希快照的关键文件同步写入：

```text
knowledge/ai_memory/project_understanding/memory_config.json
```
