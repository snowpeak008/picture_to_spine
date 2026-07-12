# 项目架构理解

> 第一次完整阅读项目后填写。此文件是跨会话的 L1 理解缓存。

## 项目定位

- 项目名称：FlashToSpine。
- 用户/使用场景：技术美术或 2D 动画师在 Windows 上把单角色素材整理为 Spine 4.2.43 生产候选，并在关键阶段人工修正和审批。
- 核心输入：用户主动选择的本地 8-bit PNG/JPEG/WebP、动作关键帧图片和动作描述文本；不内置生图、不调用公有图片 API。
- 核心输出：Rig IR、最小分层 PSD、透明 PNG、Spine JSON candidate、atlas-input manifest、PromptPack、兼容清单和 checksums。
- 主要运行方式：双击根目录 `FlashToSpineLauncher.exe`；原生 Win32 窗口宿主系统 WebView2 Evergreen Runtime。
- 固定范围：二次元类人、横版侧视、单主武器；十动作 `idle/run/jump/fall/dash/attack_01/attack_02/attack_03/hit/death`；唯一 V1 集成目标为 Spine Editor 4.2.43。

## 目录职责

| 目录 | 职责 | 修改注意事项 |
|---|---|---|
| `apps/desktop-ui` | React/TypeScript/PixiJS UI 与刚性预览 | UI 不直接访问文件系统、shell 或网络；通过版本化 IPC 请求宿主 |
| `apps/desktop/src-tauri` | 原生 Win32/WebView2 宿主、IPC、原生审批/对话框、CLI 宿主 | 目录名是历史遗留，不得据此引入 Tauri/Wry；保持导航/CSP/IPC fail-closed |
| `crates/domain` | 领域实体、规则、审批、状态机、canonical hash | 不依赖 application/adapters/delivery，不执行外部 I/O |
| `crates/application` | 用例、聚合编辑、发布快照与审批闭包 | 命令必须 revision checked、原子化并精确传播失效 |
| `crates/adapters` | 存储、图片解码、导出、CLI/凭据等边界适配 | 路径、大小、哈希、超时、外部能力状态必须 fail-closed |
| `schemas` | JSON/IPC/证据合同 | 修改时同步 Rust、TypeScript、测试和迁移说明 |
| `tools` | 构建、打包、验证、许可和记忆维护脚本 | 不得静默联网、安装、提权或夸大验证状态 |
| `docs` / `plan` | 用户/运维边界、冻结计划与原子计划 | 事实变化需要统筹更新，计划分数不等于实施证据 |
| `evidence` | 实际执行证据与哈希绑定 | 旧证据不能证明新包；敏感数据不得进入证据 |
| `knowledge/ai_memory` | 跨会话分层记忆缓存 | 每次重要同步更新 session、INDEX 和 freshness |

## 核心执行链路

```text
本地图片/文本
  → 原生选择与预检/完整解码
  → staging/CAS + Project revision
  → StyleSpec/母版人工门
  → LayerSet 分层修正人工门
  → Rig/slot/pivot/socket 人工门
  → MotionSpec/BOM/PromptPack + 关键姿势图人工门
  → 十动作轨道、Pose 与三个攻击 Hit 人工门
  → PublishSnapshot 严格预检
  → 开放格式不可变导出
  → 可选调用用户本地合法 Spine 4.2.43 Professional CLI 生成专有格式
```

## 关键模块

| 模块 | 职责 | 关键接口 |
|---|---|---|
| WebView2 宿主 | 嵌入 UI、限制导航/CSP、转发版本化 IPC | `apps/desktop/src-tauri/src/main.rs`、`ipc_host.rs` |
| UI Shell | 最近项目与各生产工作台 | `apps/desktop-ui/src/app/AppShell.tsx`、`state/projectStore.ts` |
| Domain | 项目、分层、Rig、动画、审批和远程候选合同 | `crates/domain/src/lib.rs` 及各领域模块 |
| Application | 聚合编辑、审批门和发布快照 | `crates/application/src` |
| Storage/Export | DPAPI/HMAC 项目完整性、CAS、开放格式输出 | `crates/adapters/src/storage`、`crates/adapters/src/export` |
| Spine CLI Host | 受限外部 4.2.43 操作及 provenance | `spine_cli_host.rs`、adapter CLI runner |

## 数据与配置

- 本地配置：`%LOCALAPPDATA%\FlashToSpine`，含 projects、CAS、staging、export recovery、security、WebView2；远程示例为 `config/remote-gpu.example.json`，默认禁用。
- 环境变量：构建可注入 `F2S_BUILD_INPUT_SHA256`；日常事实以脚本/配置为准，不把机器私有环境变量写入仓库。
- 运行产物：根入口、`dist/FlashToSpine-Core`、`target`、UI dist 和 evidence；发布包由 manifest/source binding 校验。
- 禁止提交：任何 secret、激活信息、角色素材/CAS、本地绝对路径、未脱敏日志、临时诊断截图。

## 已知风险

- 单张图无法恢复遮挡像素、骨骼真值、mesh/权重或动作节奏；必须依靠补充关键帧与人工门。
- 内部 Pixi 预览仅是刚性单骨工作视图，不等价于 Spine Editor/Runtime。
- 真实 Spine CLI/Editor、私有远程 GPU transport、AppContainer Worker、签名和 clean-VM 均未完成。
- GUI 全业务链尚未封账；本机启动探针只覆盖启动 DOM、窗口、运行时和关闭。
- 工具不能独立制作完整 2D 重度动作游戏，缺少玩法、引擎、hitbox、AI、关卡、音效/VFX 等系统。
