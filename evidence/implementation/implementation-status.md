# FlashToSpine 实施状态审计

审计日期：2026-07-11（Asia/Shanghai）

## 1. 裁决

```text
INTERNAL_CORE_IMPLEMENTATION_CANDIDATE
GUI_FULL_CHAIN_UNVALIDATED
EXTERNAL_CAPABILITIES_PENDING
RELEASE_NOT_AUTHORIZED
```

本文件记录源码与实际执行证据的边界，不是 `EvidenceEnvelope`，也不把计划评分折算为实施通过率。冻结原子计划为 `FINAL-20260711-135705-R2`，manifest SHA-256 为 `c3a0d964cbe34290619951dd1a8b9c32e8f06e228e9b234374611748ee76c9cc`；用户启动证据 `F2S-USER-START-AUTH-001` 只授权实施，`releaseAuthorized=false`。

## 2. 里程碑覆盖

| 里程碑 | 当前覆盖 | 未关闭边界 |
| --- | --- | --- |
| M00 决策与可行性 | 已有可行性、许可、synthetic fixture 与 Spine 能力静态证据 | 真实合法 Spine Editor/CLI 往返仍为外部 `NOT_RUN` |
| M01 工程骨架与工具链 | Rust workspace、React/TS/Pixi UI、直接 Win32/WebView2 宿主、构建入口已实现 | clean-VM 双 runner 未执行 |
| M02 领域/存储/协议 | 领域模型、revision、CAS、IPC、DPAPI CurrentUser + HMAC head/sidecar/anchor、authenticated roll-forward 已实现并有自动测试 | 旧 unsigned 项目迁移未实现；GUI 重开链未人工 E2E |
| M03 项目/导入/母版 | 创建/最近项目、本地图片原生选择、预检/完整解码/CAS、StyleSpec、母版预览与原生审批已接入 | 原生对话框到重开的完整 GUI 操作未形成自动/人工封账证据 |
| M04 分层/素材修复 | LayerSet、顺序/可见性、mask、手工 PNG 替换、重组预览、审批与精确失效传播已接入 | 隐藏像素仍必须由人工素材提供；完整 GUI 链未封账 |
| M05 Rig | 骨骼 rest/父关系、slot bone/drawKey、pivot/socket 编辑，刚性预览、诊断与审批已接入 | 不支持骨骼增删、mesh/weight/constraint 编辑或多骨骼蒙皮；Editor 往返未执行 |
| M06 动作内容 | 固定十动作 MotionSpec/BOM/PromptPack、本地关键姿势绑定、Ground Y/Scale 对齐、预览和审批已接入 | 不生图、不自动 retarget；全部素材的真实 GUI 审核未封账 |
| M07 动画/标记 | 十动作轨道/keyframe/review pose marker、Pixi 刚性预览、contact phase 内的三个 Hit marker 编辑及独立审批已接入 | transition 图、完整撤销/重做 UI、mesh/deform/完整约束预览、游戏手感验证未实现 |
| M08 导出/Spine 4.2 | Rig IR、最小 PSD、透明 PNG、Spine JSON candidate、atlas-input、PromptPack、兼容清单/checksum 与受限本地 CLI job 已接入 | `.atlas/.spine/.skel` 只由用户合法 4.2.43 CLI 生成；真实 job/Editor 视觉验收未执行 |
| M09 安全 AI/质量 | 私有 profile、领域合同、状态机、隔离存储、deterministic mock、Credential Manager adapter 已实现 | 真实 HTTPS/TLS/SPKI transport 与远程 job UI/host 未接入；Worker 为 `UNVERIFIED_EXCLUDED` |
| M10 Windows 打包 | 离线、源码绑定、事务式 Core 构建/打包、严格包白名单、双击根入口，以及带精确内部导航白名单和 DOM 就绪门禁的本机 WebView2 探针已实现 | 无安装器、更新器、签名或 clean-VM 证据 |
| M11 验收/发布候选 | 自动检查、边界文档、脱敏诊断和内部便携包流程已建立 | 六类人工门的 GUI 全链路、真实外部工具、组织 Release Gate 未完成；禁止称 Production Ready |

## 3. 已冻结的产品事实

- 输入只来自用户主动选择的本地 PNG/JPEG/WebP 与文本；没有公有图片 API，也不生成图片。
- 固定动作集合为 `idle/run/jump/fall/dash/attack_01/attack_02/attack_03/hit/death`。
- 内容域固定为二次元类人、横版侧视、单一主武器；唯一首版集成目标是 Spine Editor 4.2.43。
- 母版、分层、Rig、关键姿势图片、动作 Pose、攻击 Hit 都需要当前 revision/hash 绑定的人工审批。
- 内置 writer 永不生成 `.atlas`、`.spine` 或 `.skel`。
- 发布依赖继续由 `F2S-LIC-POLICY-001` fail closed；代码签名和发布授权不由构建脚本产生。

## 4. 证据解释规则

- 单元/集成/包测试 `PASS` 只证明对应合同实际运行成功。
- `test:webview-local` 即使通过也固定为 `LOCAL_RUNTIME_ONLY`、`cleanVm=false`，覆盖本机启动页 DOM、窗口/WebView2 启停，但不是 GUI 业务 E2E。
- synthetic Spine、mock GPU 或静态 schema `PASS` 不能升级真实外部能力。
- 未产生与当前包 SHA-256 绑定的证据前，旧探针/旧包结果一律视为过期。
- M03–M11 不能仅凭源码存在或设计分数标为全部原子 DEV/EVD 完成。
