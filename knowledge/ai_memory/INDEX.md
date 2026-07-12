# AI 会话记忆索引

> 最后更新：2026-07-12
> 缓存状态：已生成；使用前运行 `python tools/memory/check_staleness.py`，只重读变化的关键文件。

---

## 使用顺序

1. 读取本文件。
2. 读取 `project_understanding/key_files.md`，确认当前项目的关键文件清单。
3. 如需检查缓存是否过期，运行 `python tools/memory/check_staleness.py`。
4. 缓存有效时优先使用已有记忆；缓存过期时只重读变化文件，并更新相应记忆。
5. 会话结束时新增 `session_history/YYYY-MM-DD-NNN.md`，更新本索引与 freshness。

---

## 上次会话摘要

2026-07-12 首次完成项目记忆同步：确认 FlashToSpine 是 Windows 闭源商业 Spine 4.2.43 Production Assist 内部候选；同步原生 Win32/WebView2 + React/Pixi + Rust 分层架构、人工审批门、开放格式/外部 Spine 边界、当前包与验证状态，并记录 WebView2 自身导航被拦截导致黑屏的修复规则。

最新会话：`session_history/2026-07-12-001.md`。

---

## L1 项目理解缓存状态

| 文件 | 缓存状态 | 上次读取 |
|---|---|---|
| project_understanding/architecture.md | 已同步 | 2026-07-12 |
| project_understanding/key_files.md | 已同步 | 2026-07-12 |
| project_understanding/freshness.json | 有效（18 个关键文件） | 2026-07-12 |

---

## L2 代码惯例速查

详见：

- `code_conventions/patterns.md`
- `code_conventions/anti_patterns.md`

核心规则：

- 产品固定为二次元类人、横版侧视、单主武器和十动作；唯一 V1 集成目标是 Spine Editor 4.2.43。
- 输入只来自用户本地图片/文本；不内置生图，不调用公有图片 API，AI 输出只作候选。
- 依赖方向为 Domain ← Application ← Adapter/Delivery；Rust 是 revision、审批、CAS、I/O 和 IPC 权威端。
- 六类生产人工门绑定当前 revision/hash；上游变化精确使相关审批失效。
- WebView 仅允许本轮嵌入文档；导航和 React `.app-shell` DOM 就绪后才能发布正式窗口标题。
- 内置 writer 不生成 `.atlas/.spine/.skel`；真实 Spine、远程 GPU、签名等未运行时保持 `NOT_RUN/EXTERNAL`。
- 发布依赖只允许经审计的宽松许可；新增依赖先更新清单和测试。
- 当前是内部 Core 候选，`releaseAuthorized=false`；本地/静态/mock PASS 不能升级为 clean-VM、真实外部能力或 Production Ready。

---

## L3 决策记录

详见：

- `decisions/architecture.md`
- `decisions/open_questions.md`

---

## L4 待办决策

当前重点：GUI 全业务链验收方案、真实 Spine 4.2.43 往返资源、私有远程 GPU transport 设计、旧 unsigned 项目迁移、clean-VM/签名/Release Gate，以及记忆写入的长期触发策略。详见 `decisions/open_questions.md`。
