# 架构决策记录

> 记录已经确定、后续会话应继续遵守的决策。

### 2026-07-11：产品边界与唯一集成目标

**状态**：Accepted

**背景**：单张图无法恢复遮挡素材、Rig 真值和完整动作设计，产品需要明确可交付边界。

**决策**：V1 固定为二次元类人、横版侧视、单主武器和十动作；只面向 Spine Editor 4.2.43；定位为人工可修正/审批的生产资料工作台。

**影响**：不承诺 Unity/Godot/Cocos/自研 Runtime、完整游戏、任意角色或其他 Spine patch。

### 2026-07-11：原生 Win32/WebView2 与 Rust 权威端

**状态**：Accepted

**背景**：Windows 桌面候选需要小型、可审计宿主并隔离 UI 权限。

**决策**：使用原生 Win32 + `webview2-com`；React/TS/Pixi 嵌入 exe；Rust 负责 revision、审批、CAS、文件 I/O 和 IPC。`src-tauri` 仅保留历史路径名。

**影响**：UI 无通用 shell/fs/http；CSP 默认拒绝网络；跨层合同必须同步测试。

### 2026-07-11：人工门、候选状态与精确失效传播

**状态**：Accepted

**背景**：AI、默认 Rig 和动作估计不能成为生产真值。

**决策**：母版、分层、Rig、关键姿势图片、十动作 Pose 与三个 Hit 均绑定 revision/hash，由人批准或拒绝；上游修改只按依赖使相关审批失效。

**影响**：AI/PromptPack/远程结果永远先作为 candidate；导出前重新计算完整审批闭包。

### 2026-07-11：专有 Spine 格式保持外部

**状态**：Accepted

**背景**：Spine Editor/CLI 与专有格式受用户许可证和精确版本约束。

**决策**：内置 writer 永不生成 `.atlas/.spine/.skel`；只可调用用户本地合法 Professional/适用 Enterprise 4.2.43，并记录一次性确认与 provenance。

**影响**：真实往返未运行前状态保持 `NOT_RUN/EXTERNAL`；产品包不包含 Spine 软件、Runtime 或激活信息。

### 2026-07-12：WebView 启动采用精确导航与 DOM 就绪门禁

**状态**：Accepted

**背景**：旧导航规则拦截自身 `NavigateToString` 文档，造成黑屏；旧探针只看窗口会误报成功。

**决策**：只接受 `about:blank` 与本轮 HTML 精确 data URI；先验证 `NavigationCompleted` 和 React 根节点/`.app-shell`，再发布正式窗口标题。

**影响**：启动失败显示稳定错误并退出；本机探针可证明启动 DOM，但仍不代表 GUI 全业务链。
