# R3a 独立逐文件评审：06–12

## 1. 评审绑定

| 字段 | 值 |
| --- | --- |
| snapshotId | `R3A-20260711-083804-FINAL2` |
| 只读输入根 | `plan/reviews/snapshots/R3A-20260711-083804-FINAL2/plan/` |
| manifest SHA-256 | `3d04d361f1f19485006a21e8f7cb4b8378c87f55847329a5c20edbc3c623088d` |
| mechanical audit SHA-256 | `9117469b7b0b328f68424b0ab109eefd4f0e053f4021e68b1906ae975317b50f` |
| mechanical audit 结果 | `29/29 PASS` |
| reviewer canonical ID | `F2S-REVIEWER-R3A-PRODUCT-001` |
| 开始时间（Asia/Shanghai） | `2026-07-11T08:38:50.6462975+08:00` |
| 结束时间（Asia/Shanghai） | `2026-07-11T08:43:54.1741663+08:00` |

审阅者使用 22 号文件的 12 维量表。维度顺序及满分固定为：范围覆盖 12、可行性与证据状态 10、跨文档一致性 10、稳定 ID 与追踪 10、可测试与验收 8、失败/安全/许可 6、清晰无歧义 4，以及架构类的边界契约 8、模式取舍 8、状态/并发/版本 8、性能/恢复 8、替换/降级 8。07 号环境文件的后五维依次解释为干净机复现、版本锁定、诊断、回滚、离线安全交付。

逐维通过 floor 为 `[10,8,8,8,7,5,4,7,7,7,7,7]`；总分须不低于 95，且不得存在 P0/P1。本报告中的 P2 证据状态不把尚未实测的技术能力提升为 `VERIFIED`。

## 2. 汇总判定

| 文件 | docId | 12 维整数向量 | 总分 | floor | P0 | P1 | P2 | 判定 |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | --- |
| 06 | `F2S-DOC-ARCH-001` | `[12,9,10,10,8,6,4,8,8,8,8,8]` | 99 | PASS（最低归一化 90%） | 0 | 0 | 1 | PASS |
| 07 | `F2S-DOC-ENV-001` | `[12,8,10,10,8,6,4,8,7,8,8,8]` | 97 | PASS（最低归一化 80%） | 0 | 0 | 1 | PASS |
| 08 | `F2S-DOC-RENDER-001` | `[12,9,10,10,8,6,4,8,8,8,8,8]` | 99 | PASS（最低归一化 90%） | 0 | 0 | 1 | PASS |
| 09 | `F2S-DOC-DOMAIN-001` | `[12,8,10,10,8,6,4,8,8,8,8,8]` | 98 | PASS（最低归一化 80%） | 0 | 0 | 1 | PASS |
| 10 | `F2S-DOC-PIPE-001` | `[11,9,10,10,7,6,4,8,8,8,8,8]` | 97 | PASS（最低归一化 87.5%） | 0 | 0 | 1 | PASS |
| 11 | `F2S-DOC-STORE-001` | `[12,8,10,10,8,6,4,8,8,8,8,8]` | 98 | PASS（最低归一化 80%） | 0 | 0 | 1 | PASS |
| 12 | `F2S-DOC-IPC-001` | `[12,8,10,10,8,6,4,8,8,8,8,8]` | 98 | PASS（最低归一化 80%） | 0 | 0 | 1 | PASS |

七份文件均达到总分和逐维 floor，且没有 P0/P1、阻塞 TBD、许可硬冲突或隐私硬冲突。本批次总体判定为 `PASS`。

## 3. 逐文件评审

### 3.1 06 — 系统架构与设计模式

- docId：`F2S-DOC-ARCH-001`
- snapshot-relative path：`plan/06-系统架构与设计模式.md`
- input SHA-256：`650323e9c2af08407a4b8840766acd9ba3b5b736836f68dc94011f47452d49f1`
- 12 维向量：`[12,9,10,10,8,6,4,8,8,8,8,8]`
- 总分 / floor：`99 / PASS`
- P0：无。
- P1：无。
- P2：`F2S-R3A-B06-P2-001`——依赖方向、最小权限、Worker 终止隔离和端到端纵切仍是计划门禁，尚无本轮冻结输入之外的实现/实测证据；该事实已由第 11 节“验收预算而非已证明能力”和 `F2S-TST-060`–`066` 明确约束，因此只在“可行性与证据状态”扣 1 分，不构成设计阻断。

第三人称结论：审阅者认为该文件已经给出可实施且没有第二事实源的六边形架构；其证据为第 3–5 节冻结 Rust 权威、Application/Infrastructure 端口与唯一 Repository 映射，第 6–8 节冻结依赖方向、工程结构和模式适用边界，第 9–13 节覆盖补偿、安全、性能、测试与许可；因此除尚待实现证据外，其余维度不扣分。

具体章节证据：§3 `F2S-ADR-ARCH-001`–`005`；§4 `F2S-CMP-001`–`008`；§5 `F2S-IFC-001`–`004`；§8 模式取舍表；§9 错误和补偿；§10 最小权限；§11 量化预算；§12 `F2S-TST-060`–`066`；§13 发布许可分类。

### 3.2 07 — Windows 环境配置与工具链

- docId：`F2S-DOC-ENV-001`
- snapshot-relative path：`plan/07-Windows环境配置与工具链.md`
- input SHA-256：`fc9f0e3f796c393b6d347a7feebe58b75d1b22ce54e54f2806d8f1e7c39a96d5`
- 12 维向量：`[12,8,10,10,8,6,4,8,7,8,8,8]`
- 总分 / floor：`97 / PASS`
- P0：无。
- P1：无。
- P2：`F2S-R3A-B07-P2-001`——Node/npm、Rust 和 Python 的精确 patch、实际锁文件及顶层命令实跑结果要到 `F2S-DEV-M00-001`/`F2S-EVD-M00-001` 才冻结；当前仅有 LTS/系列候选和 fail-closed 禁止浮动版本规则。该项分别在“可行性与证据状态”扣 2 分、“版本锁定”扣 1 分；M00 前明确为 `UNVERIFIED/planned`，故不是 P1。

第三人称结论：审阅者认为该环境计划可以指导清洁 Windows 机器搭建、独立 Runtime Pack、离线安装与双击入口；其证据为第 2–5 节的平台/工具/硬件矩阵，第 7 节稳定工程命令及凭据边界，第 8–10 节独立打包、安装入口和 CI，第 11–13 节故障、测试与许可约束；因此只有尚待 M00 锁定和实跑的版本证据被扣分。

具体章节证据：§2 Win11 P0/Win10 P1；§3 精确 patch 的 M00 退出条件；§5 `CapabilityReport` 与 `UNVERIFIED|VERIFIED|FAILED`；§7 planned 命令；§8 Runtime Pack 审计；§9 `FlashToSpine.cmd` 和 WebView2 缺失路径；§10 clean-vm-smoke；§12 `F2S-TST-070`–`079`；§13 PSF/CUDA/NSIS/WebView2/Spine 分类。

### 3.3 08 — 前端渲染与编辑器内核

- docId：`F2S-DOC-RENDER-001`
- snapshot-relative path：`plan/08-前端渲染与编辑器内核.md`
- input SHA-256：`5d8280faecaa2610b817bdae808d4a27502bee1abc9be691c128a33d85851114`
- 12 维向量：`[12,9,10,10,8,6,4,8,8,8,8,8]`
- 总分 / floor：`99 / PASS`
- P0：无。
- P1：无。
- P2：`F2S-R3A-B08-P2-001`——WebView2/Pixi 的目标 GPU、DPI、10,000 关键帧、context-loss 与时间转换 golden 尚未实跑；文件已用第 14 节预算、第 15 节恢复流程及 `F2S-TST-080`–`089` 给出精确证据出口，因此仅在“可行性与证据状态”扣 1 分。

第三人称结论：审阅者认为该文件清楚隔离 React、Pixi 和 Rust 权威，并把高频预览、提交语义、时间精度和降级恢复写成可测试契约；其证据为第 5–6 节状态/快照/patch/intent 接口，第 7–13 节坐标、编辑器和诊断语义，第 14–18 节量化性能、context-loss、可访问性、失败矩阵及测试；因此设计本身不需要整改。

具体章节证据：§6 `RenderSnapshot/RenderPatch/EditorIntent` 的 epoch/revision 拒绝规则；§7 坐标/DPI；§9–10 mask/Rig 权威重放；§11 有理 tick 与稳定 keyframe ID；§12 审批失效；§14 量化预算；§15 context-loss；§16 A11Y；§17 失败矩阵；§18 exact tests。

### 3.4 09 — 领域逻辑与工作流编排

- docId：`F2S-DOC-DOMAIN-001`
- snapshot-relative path：`plan/09-领域逻辑与工作流编排.md`
- input SHA-256：`717e15026ebf6656984bbcbe3b8a4c4666833e42282889ab5bbc1d83d808b7ff`
- 12 维向量：`[12,8,10,10,8,6,4,8,8,8,8,8]`
- 总分 / floor：`98 / PASS`
- P0：无。
- P1：无。
- P2：`F2S-R3A-B09-P2-001`——Gate/Approval/Priority/Actor/Waiver/Clock 的大量 schema、JCS 双哈希、签名与跨语言 property/golden 测试均已精确定义，但在当前计划快照中尚没有生成代码和执行结果；复杂度使“可行性与证据状态”扣 2 分。所有失败路径均 fail closed，且 `F2S-TST-090`–`099` 给出退出条件，所以该证据缺口不升级为 P1。

第三人称结论：审阅者认为该文件已经把人工门禁、稳定审批、优先级防降级、幂等和离线时钟回拨边界闭合到可实现的领域契约；其证据为第 4–8 节五维状态、依赖闭包和基础 Gate，第 9–10 节审批/waiver/clock 与失效，第 12–15 节 Job 仲裁、命令幂等、不变量和失败语义，第 16 节覆盖恶意与竞态向量；因此不发现逻辑 P0/P1。

具体章节证据：§4 五维状态唯一映射；§6 失效传播；§7 人工门；§8 严格 ULID、RuleCatalog/Priority/Defect、wire/JCS/hash/DecisionPolicy；§9 append-only Approval/Clock 与 current dynamic validation；§12.4 终态仲裁；§13 commandKind+payload 幂等；§14 不变量；§15 零事件失败；§16 `F2S-TST-090`–`099`。

### 3.5 10 — 本地图像处理与素材规划管线

- docId：`F2S-DOC-PIPE-001`
- snapshot-relative path：`plan/10-本地图像处理与素材规划管线.md`
- input SHA-256：`fadb5637fe8cefaccc52cc1e2fead928ddc650dc72254410272c8f9794b6a2e5`
- 12 维向量：`[11,9,10,10,7,6,4,8,8,8,8,8]`
- 总分 / floor：`97 / PASS`
- P0：无。
- P1：无。
- P2：`F2S-R3A-B10-P2-001`——§5.1 要求对文件大小、像素总量和压缩比设置“可配置硬限”，§18 `F2S-TST-100` 要求“超限尺寸”拒绝，但本文及其被核对的快照依赖未冻结默认/绝对上限和边界值。该缺口使第三方尚不能直接构造上限±1 fixture，故在“范围覆盖”扣 1 分、“可测试与验收”扣 1 分；原子计划必须先冻结常量、配置可收紧规则和边界语料，再实现解码入口。

第三人称结论：审阅者认为该管线正确坚持母版像素真值、动作需求先行、手工可完成和 AI 只产 candidate；其证据为 P00–P90 的阶段闭环、Pixel/Generation Provenance 双门、Stage/checkpoint/idempotency 接口、8GB 显式降级及测试矩阵；但导入资源上限还需在原子开发计划中量化，因此做两项真实扣分而不阻断 R3a。

具体章节证据：§5 magic bytes/解压炸弹/PSD 无副作用拒绝；§6 immutable source；§8 representation rules；§9 coverage/PromptPack；§10–12 segmentation/provenance/recomposition；§13 feedback loop；§14 Stage 契约；§15 H1 降级；§16 模型 manifest；§17 恢复；§18 `F2S-TST-100`–`109`。整改落点为 §5.1 与 `F2S-TST-100` 的原子任务参数化。

### 3.6 11 — 数据模型、项目存储与迁移

- docId：`F2S-DOC-STORE-001`
- snapshot-relative path：`plan/11-数据模型项目存储与迁移.md`
- input SHA-256：`adde33882c2a30a6a67e783ca44f6b1309608c6d9d6b95aa135b33a5c911fc7a`
- 12 维向量：`[12,8,10,10,8,6,4,8,8,8,8,8]`
- 总分 / floor：`98 / PASS`
- P0：无。
- P1：无。
- P2：`F2S-R3A-B11-P2-001`——Windows/NTFS 上的 flush、同卷 rename/replace、指针切换、防病毒占用和杀进程原子性仍是 `UNVERIFIED`，尚无 `F2S-TST-111` 实机结果；该能力是存储承诺的关键实现风险，故“可行性与证据状态”扣 2 分。§8.1 和完成定义明确在证据前不得发布，避免形成 P1 设计漏洞。

第三人称结论：审阅者认为该文件已形成项目目录真值、不可变 CAS、registry/clock 可达根、单写者、可重建 redb、链式迁移和外部发布恢复的一致存储模型；其证据为第 6–8 节 manifest/CAS/提交顺序，第 9–13 节锁、索引、接口和迁移，第 14–17 节 PublishAttempt、恢复和实机故障测试；因此仅保留 Windows 原子能力的实测扣分。

具体章节证据：§6 RevisionManifest/ClockPointer；§7 CAS 双哈希与 GC roots；§8 staging→CAS→manifest→pointer 及 journal；§8.3 Job 权威链；§9 单写者；§10 redb 非真值；§11.1 canonical sandbox；§12 唯一 Repository；§13 fail-closed migration；§14 append-only PublishAttempt；§16 故障矩阵；§17 `F2S-TST-110`–`120`。

### 3.7 12 — IPC 协议、后台 Worker 与私有远程 GPU

- docId：`F2S-DOC-IPC-001`
- snapshot-relative path：`plan/12-IPC协议后台Worker与远程GPU.md`
- input SHA-256：`06c8ffba3df992a3b345988330a4547531da08a8080d62a17676617206605e82`
- 12 维向量：`[12,8,10,10,8,6,4,8,8,8,8,8]`
- 总分 / floor：`98 / PASS`
- P0：无。
- P1：无。
- P2：`F2S-R3A-B12-P2-001`——`windows-appcontainer-v1` 对签名 Python/CUDA Runtime Pack、GPU DLL、Job Object、ACL 与零 egress 的组合可行性尚未在目标 Windows 11 机器实测；这是重要实现风险，故“可行性与证据状态”扣 2 分。§9.4 以 `F2S-GATE-IPC-SANDBOX-001` 规定任何控制失败即物理禁用 LocalPythonProvider/Worker Pack，手工核心继续，因此没有可绕过的 P1。

第三人称结论：审阅者认为该协议完整定义了有界 NDJSON、session/seq/resync、取消与 success 仲裁、隔离 Artifact 提升、私有端点重审批和 OS 级 Worker 沙箱；其证据为第 3–7 节协议/资源/状态/提升，第 8–10 节稳定错误、私有远程信任和隐私，第 11–12 节兼容与恶意测试；因此仅对尚未实测的 AppContainer+GPU 组合扣分。

具体章节证据：§3 路径与不传 Base64；§4 固定资源上限和 handshake；§5 seq/resync/terminal arbitration/cancel；§6 幂等与显式 OOM；§7 Rust 验证后才产生 unbound Artifact；§9.2 upload approval/receipt；§9.3 TLS/证书变化；§9.4 六项 OS 隔离与 hard gate；§12 IPC/RGPU exact tests。

## 4. 独立性与输入完整性声明

审阅者 `F2S-REVIEWER-R3A-PRODUCT-001` 声明：其未参与 06–12 七份文档的作者工作；本轮逐文件判断只读取 `R3A-20260711-083804-FINAL2` 冻结快照、该快照的 manifest、机械审计及同一快照内的 22 号量表/交叉引用，没有把 live `plan/06`–`plan/12` 作为评分输入。审阅者未修改 snapshot、未修改 00–24，仅新增本报告。

报告结论：`06–12 = PASS`。无需因本批次重启 R3a；`F2S-R3A-B10-P2-001` 应在原子开发计划的导入安全任务中先行量化，其余 P2 均由既有证据门在实现阶段关闭，关闭前不得把相应能力标为 `VERIFIED`。
