# FlashToSpine 项目记忆

## 1. 用途与当前裁决

本文件是项目独立、长期、供后续会话恢复的事实摘要。它不替代 `plan/`、`plan/devplan/`、运行证据或用户最新指令；冲突时按“用户最新明确指令 → 实际运行证据 → 冻结计划”的顺序处理。

当前实施裁决：

```text
executionState=AUTHORIZED_FOR_IMPLEMENTATION
implementationAuthorized=true
implementationStatus=INTERNAL_CORE_IMPLEMENTATION_CANDIDATE
guiFullChain=UNVALIDATED
externalCapabilities=NOT_RUN_OR_PENDING
releaseAuthorized=false
```

不得把当前候选称为全部 80 个原子 DEV/EVD 已封账、Production Ready、已通过 clean-VM、已兼容真实 Spine 或已获商业发布授权。

## 2. 冻结产品边界

- 产品：闭源商业 Windows 桌面 Production Assist，不是单图一键成片器，也不是游戏引擎。
- 内容：二次元类人、横版侧视、单一主武器；动作键固定且顺序固定为 `idle/run/jump/fall/dash/attack_01/attack_02/attack_03/hit/death`。
- 输入：只接收用户主动选择的本地图片与文本；可信图片格式为通过预检和完整解码的 8-bit PNG/JPEG/WebP。
- 图片/AI：程序不生成图片、不调用公有图片 API；可生成动作描述、素材 BOM 和 PromptPack，所有 AI 结果只可作为候选。
- 人工门：母版、分层、Rig、关键姿势图片、十动作 Pose、三个攻击 Hit 必须可查看、修改、批准或拒绝，并绑定当前 revision/hash。
- Spine：唯一目标是精确 4.2.43。内置输出为 Rig IR、最小 PSD、透明 PNG、Spine JSON candidate、atlas-input manifest、PromptPack、兼容清单和 checksums。
- 专有格式：`.atlas/.spine/.skel` 只能调用用户本地合法的 Spine Professional 或适用 Enterprise 4.2.43 生成；内置 writer 永不生成。
- 集成目标：V1 只面向 Spine Editor，不把 Unity、Godot、Cocos 或自研 Runtime 作为首版验收目标。
- 许可：发布依赖只允许经审计的 MIT、Apache、BSD 等宽松许可；未知、copyleft 或来源不明项 fail closed。

## 3. 计划与授权事实

- 总计划冻结版本：`R3B-20260711-085312-FINAL`。
- 原子计划冻结快照：`FINAL-20260711-135705-R2`。
- 原子 manifest SHA-256：`c3a0d964cbe34290619951dd1a8b9c32e8f06e228e9b234374611748ee76c9cc`。
- FINAL audit SHA-256：`a46124e23c647afc64292d02e3d083ccb3fcb9c8baeb535905bf22a6b18a4f56`。
- Detached final review SHA-256：`4b791307403ed1e670dc67c7d74135a6d7d4f78af375622b8505e004437d291b`。
- 用户开始证据：`plan/devplan/reviews/user-start-authorization.md`，证据 ID `F2S-USER-START-AUTH-001`；用户已明确发送“开始执行”。
- 该授权允许实现、测试和构建内部候选，但 `releaseAuthorized=false` 始终有效。
- Detached FINAL 中记录的 `project_memory_sha256` 绑定的是用户开始前的设计期记忆；本文件在获得实施授权后按实际结果重写，因此当前哈希不同是预期事实，不得把历史哈希当成实施期零漂移要求。

## 4. 当前实现架构

- 桌面：原生 Win32 + `webview2-com` 直接宿主系统 WebView2 Evergreen Runtime；不是 Tauri/Wry。`apps/desktop/src-tauri` 只是历史目录名。
- 启动安全：`NavigateToString` 只允许 `about:blank` 与本次嵌入 HTML 对应的精确 data URI；最终窗口标题仅在导航完成且 React 根节点/`.app-shell` DOM 就绪后发布，失败时以 `F2S-BOOT-NAVIGATION` 或 `F2S-BOOT-DOM` 明确退出，避免静默黑屏。
- 前端：React + TypeScript + PixiJS，esbuild 打包后嵌入同一 exe；WebView 无通用 shell/fs/http，CSP 默认拒绝网络。
- 后端：Rust 分层为 Domain ← Application ← Adapter/Delivery；Rust 是项目 revision、审批、CAS、原生对话框、文件写入和 IPC 的权威端。
- 时间：Rig IR/动画内部使用整数 tick，`timeBase=1/30000`；只有适配器边界可转换为秒。
- 本地数据：项目、CAS、staging、恢复记录、配置和 WebView2 用户数据位于 `%LOCALAPPDATA%\FlashToSpine`；浏览器缓存不会写在 exe/便携包旁。
- 项目完整性：生产入口使用 DPAPI CurrentUser 包装的 256-bit 密钥，以及 HMAC ProjectHead、不可变 signed revision sidecar 和高水位 anchor；读取只允许受验证的 authenticated roll-forward。

## 5. 已接入的 Core 工作流

- 创建/打开/最近项目；最近项目按有效 `head.json` 修改时间排序，单个损坏项目隔离。
- 本地图片原生选择、格式/大小/像素/完整解码/压缩比预检、staging、CAS 和有界安全预览。
- StyleSpec、母版候选及完整 payload 的原生人工审批。
- LayerSet 的添加/删除/排序/可见性、mask stroke、人工 PNG 替换、重组与像素 provenance 审批。
- Rig 候选的现有骨骼 X/Y/rotation/Scale X/Scale Y、父关系、slot bone/drawKey、pivot 和主武器 socket 编辑；mesh/weight/constraint 只读诊断；Pixi 只做刚性单骨预览。
- 固定十动作 MotionSpec、素材 BOM、离线 PromptPack、本地关键姿势图绑定、Ground Y/统一 Scale 对齐、有界预览和逐图审批。
- 十动作动画轨道/keyframe、review pose marker；三个攻击动作的唯一 Hit marker 可在 MotionSpec `contact` phase 内编辑并绑定当前主武器 socket。Hit 修改只使对应 Hit 审批失效，Pose 保持；上游变化按精确依赖传播失效。
- 开放格式发布快照、严格导出预检、不可变目录提交、checksums 与恢复记录。
- 本地 Spine CLI 设置与 `IMPORT_PROJECT/PACK_ATLAS/EXPORT_BINARY` 异步宿主合同、一次性原生许可确认、精确 4.2.43 探针与输出 provenance；真实合法 CLI 尚未执行。
- 私有远程 GPU profile/领域合同/状态机/隔离存储/deterministic mock/Credential Manager adapter；真实 HTTPS/TLS/SPKI transport 与 job UI/host 未接入。
- 脱敏诊断 JSON 原生导出，不包含图片、PromptPack 正文、secret、用户名或绝对路径。

## 6. 当前内部包与验证

根双击入口：`FlashToSpineLauncher.exe`。

包目录：`dist/FlashToSpine-Core`。

最终一次源码绑定构建（2026-07-12）：

- EXE/root entry SHA-256：`658718d055374000e0cf008cb0b1427e92986b8f898ce92a64fccc687bddff82`。
- build input SHA-256：`7a6906f08c8794615e9de6a04c7a4801f1e2db355856190960dbc9ae5cf39590`。
- source tree SHA-256：`402a244d37d4ee6aaf23529f2913bc54fc4cbdaff683c95d7b530f2d13c097e9`。
- UI bundle SHA-256：`42070ac60ba951906a320e40a16e9b700114da1a085e7cd51f86aa3ef0480614`。
- package manifest SHA-256：`648b6976f011a4841e57c7c5db9291628109e6d019d1f082edbf843eca32b74d`。
- 构建：`--locked --offline`、隔离 snapshot、事务提交；包验证 `PASS`。
- 本机 WebView2 探针：与上述 EXE 哈希绑定，`PASS / LOCAL_RUNTIME_ONLY / cleanVm=false`；内部导航与 React 根节点/`.app-shell` DOM 就绪门禁、顶层窗口、Chrome/Render 子窗口、响应性和 `WM_CLOSE` 正常退出通过。
- 跨进程存储：独立 create/open 测试进程验证固定测试密钥下 HMAC/CAS 持久化及错误密钥 MAC fail-closed；不覆盖 DPAPI provisioning 或 GUI E2E。
- 工具链、格式、架构 lint、类型、Node/UI、Rust workspace、集成、Spine 静态合同、许可清单和 CI 合同均在该轮通过。

证据入口：

- `evidence/implementation/final-validation.json`
- `evidence/implementation/webview2-local-startup.json`
- `evidence/implementation/license-final.json`
- `evidence/implementation/implementation-status.md`

## 7. 尚未关闭的边界

- GUI 全业务链未封账：尚无从原生图片对话框开始，穿过六类人工门、关闭/重开、导出并复核的完整 Windows GUI E2E 证据。
- Rig 不支持骨骼增删、mesh 顶点编辑、weight painting、约束编辑或多骨骼蒙皮；动画没有 transition 图、完整撤销/重做 UI 或批量 retarget。
- Pixi 预览不是 Spine Runtime/Editor 等价渲染，不验证 mesh/deform、完整约束、像素一致性或游戏手感。
- 私有远程 GPU 真实 transport/job 未接入；当前网络尝试为 0，必须保持 `NOT_RUN/EXTERNAL`。
- AppContainer AI Worker 未包含，状态为 `UNVERIFIED_EXCLUDED`。
- 真实 Spine 4.2.43 CLI/Editor 往返、Editor 人工视觉验收未运行；synthetic 测试不能代替。
- 没有安装器、更新器、代码签名、两台 clean-VM runner 或组织 Release Gate。
- 本工具不能独立产生完整 2D 重度动作游戏；没有输入/连招/取消、hitbox/hurtbox、伤害、AI、关卡、物理、引擎 Runtime、音效/VFX 或平台发布。

## 8. 后续恢复顺序

后续会话依次读取：

1. 本文件；
2. `evidence/implementation/implementation-status.md`；
3. `evidence/implementation/final-validation.json`；
4. `plan/devplan/reviews/user-start-authorization.md`；
5. `plan/devplan/00-原子计划索引与执行治理.md`；
6. `plan/devplan/13-原子任务DAG与追踪矩阵.md`；
7. `docs/limits/known-limitations.md` 与 `docs/maintenance/operations.md`。

优先后续工作是：完成 Windows GUI 全链路人工/自动验收；若用户要启用私有远程 GPU，则先设计并评审真实 TLS/SPKI transport 和一次性外传审批；真实 Spine、签名、clean-VM 与发布仍需外部资源和独立授权。任何新证据都必须绑定当时的 EXE/build input 哈希，不能复用本轮哈希证明未来包。
