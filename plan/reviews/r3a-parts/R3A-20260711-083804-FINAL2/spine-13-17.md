---
schemaVersion: "1.0.0"
phase: R3A
snapshotId: R3A-20260711-083804-FINAL2
reviewerId: F2S-REVIEWER-R3A-SPINE-001
reviewedRange: "13-17"
manifestSha256: 3d04d361f1f19485006a21e8f7cb4b8378c87f55847329a5c20edbc3c623088d
mechanicalAuditSha256: 9117469b7b0b328f68424b0ab109eefd4f0e053f4021e68b1906ae975317b50f
mechanicalAuditSummary: "29/29 PASS"
reviewStartedAtAsiaShanghai: "2026-07-11T08:39:52.5204589+08:00"
reviewEndedAtAsiaShanghai: "2026-07-11T08:42:49.5388480+08:00"
overallP0Count: 0
overallP1Count: 0
overallVerdict: PASS
---

# R3a 独立审阅分片：13–17

## 1. 审阅声明与评分口径

审阅者 `F2S-REVIEWER-R3A-SPINE-001` 声明：其未参与本分片五份文档的作者工作；本次只以 `plan/reviews/snapshots/R3A-20260711-083804-FINAL2/plan/` 中 13–17 号文件的冻结字节作为内容评分输入，没有用 live `plan/` 文件补充或修正结论，也没有修改 snapshot 或 00–24 号文件。

12 维顺序固定为：范围覆盖、可行性与证据状态、跨文档一致性、稳定 ID 与追踪、可测试与验收、失败/安全/许可、清晰无歧义，以及该文档类别对应的五个领域维度。满分向量为 `[12,10,10,10,8,6,4,8,8,8,8,8]`；逐维 80% 向上取整后的最低通过向量为 `[10,8,8,8,7,5,4,7,7,7,7,7]`。总分还必须不低于 95，且没有 P0/P1。

外部程序、操作系统隔离、干净机、CLI、许可证或法务材料尚未实际取得时，审阅者只评价计划是否准确标记和门禁这些事实；这些缺口不会因本次文档 PASS 自动变为实现或能力 `VERIFIED`。

## 2. 逐文件结果

### 2.1 13 — Rig IR、PSD、PNG 与 Spine 4.2 导出设计

- `docId`: `F2S-DOC-EXPORT-001`
- `path`: `plan/reviews/snapshots/R3A-20260711-083804-FINAL2/plan/13-RigIR-PSD-PNG-Spine42导出.md`
- `inputSha256`: `1a06a0c9587b71a713a4d3f1b5351bdd84e229fbc844868c664d1a91fe2385a0`
- `reviewStartedAtAsiaShanghai`: `2026-07-11T08:39:52.5204589+08:00`
- `reviewEndedAtAsiaShanghai`: `2026-07-11T08:42:49.5388480+08:00`
- `domainDimensions`: 边界契约、模式取舍、状态/并发/版本、性能/恢复、替换/降级
- `scoreVector`: `[12,9,9,10,8,6,4,8,8,8,8,8]`
- `total`: `98/100`
- `dimensionFloor`: `PASS`；逐维均不低于 `[10,8,8,8,7,5,4,7,7,7,7,7]`
- `P0`: `[]`
- `P1`: `[]`
- `P2`: `[F2S-R3A-SPINE-13-P2-001]`
- `externalOrExecutionPending`: `[F2S-OQ-SPN-001, PSD writer/reader/Editor fixture evidence, Spine Professional 4.2.43 CLI round-trip evidence]`
- `verdict`: `PASS`

精确问题：

- `F2S-R3A-SPINE-13-P2-001`：§13.9 `F2S-EXP-RT-003` 的示例同时给出 `exportMode="release"`、L0–L3 全部 `pass`，却把 `cli.capabilityStatus` 写为 `UNVERIFIED`。正文 §13.7、§13.9 和 §13.10 已清楚规定 Release 只接受已验证能力，因此该示例不会放行真实发布，但示例组合可能令实现者误解 CLI 能力状态与本轮 L2 证据的关系。后续应把示例改为自洽的 release/verified 组合，或明确 `cli.capabilityStatus` 与 feature evidence status 是不同字段。

章节证据：

- §13.1–13.3 明确 Rig IR、PSD/PNG、内置 JSON/atlas-input manifest 与用户本地 CLI 的 writer ownership，禁止内置 `.atlas/.spine/.skel` fallback。
- §13.4 将坐标、名称、i64 tick、half-even、binary32 碰撞和领域不变量落到可执行契约。
- §13.5 以 writer/独立 reader/Editor UI 三层验证 PSD，避免用同一实现自证。
- §13.7、§13.9、§13.10 分离 Candidate、Release、L0–L3 和原子导出/外部 PublishAttempt；`NOT_RUN` 不会汇总成 PASS。
- §13.8、§13.12 明确当前 CLI/授权证据缺失，并给出固定 fixture、负测、provenance 和稳定验收映射。

第三人称结论：审阅者认为该文档已形成可实施、可降级且许可边界清楚的导出设计。其唯一扣分是兼容清单示例中的状态组合歧义；该问题为 P2，不改变 Candidate/Release 正文门禁，也不构成提前宣称 4.2.43 已验证。

### 2.2 14 — 安全、隐私、许可证与软件供应链设计

- `docId`: `F2S-DOC-SEC-001`
- `path`: `plan/reviews/snapshots/R3A-20260711-083804-FINAL2/plan/14-安全隐私许可证与供应链.md`
- `inputSha256`: `a06bbfdd243524628607523ae94e2e7bab7cfdee4d6bf17db5a38e5576f1790d`
- `reviewStartedAtAsiaShanghai`: `2026-07-11T08:39:52.5204589+08:00`
- `reviewEndedAtAsiaShanghai`: `2026-07-11T08:42:49.5388480+08:00`
- `domainDimensions`: 威胁/隐私、供应链/许可、证据、安全响应、残余风险
- `scoreVector`: `[12,9,10,10,8,6,4,8,8,7,8,8]`
- `total`: `98/100`
- `dimensionFloor`: `PASS`；逐维均不低于 `[10,8,8,8,7,5,4,7,7,7,7,7]`
- `P0`: `[]`
- `P1`: `[]`
- `P2`: `[]`
- `externalOrExecutionPending`: `[windows-appcontainer-v1 OS probe evidence, Esoteric written boundary confirmation, model weight/commercial redistribution evidence, Windows code-signing custody evidence, third-party legal review]`
- `verdict`: `PASS`

章节证据：

- §14.2–14.6 对 D0–D3、Rust/Worker/CLI、文件边界和自托管 HTTPS Provider 建立默认拒绝模型。
- §14.4 将商业 D2 Worker 的唯一发布配置固定为 `windows-appcontainer-v1`，并把网络、ACL、Job、句柄与恶意逃逸探针列为硬门。
- §14.7–14.8 由唯一许可政策管理代码、权重、数据、字体、Spine 和 SBOM/provenance，未知项 fail closed。
- §14.9–14.11 对组织 enrollment、三 credential、算法 registry、bundle 回滚、离线 freshness 边界和漏洞 SLA 给出可执行正负测试。
- §14.12–14.14 给出事件响应、更新根失陷处置、稳定验收映射，并明确外部证据尚待取得且本文不替代法律意见。

第三人称结论：审阅者认为该文档完整覆盖本地机密素材、可选远端、Worker 隔离、供应链、Spine 许可、多签与事件响应。扣分仅来自真实 OS/法务/签名材料尚未执行或取得；正文已正确保持这些能力为外部待证状态，因此没有设计级 P0/P1。

### 2.3 15 — 测试、质量、性能与可观测性

- `docId`: `F2S-DOC-QUALITY-001`
- `path`: `plan/reviews/snapshots/R3A-20260711-083804-FINAL2/plan/15-测试质量性能与可观测性.md`
- `inputSha256`: `e0c0f065c786ac98d113f39a25a1d9df9359a7b9afd44b09b8477433119bcbe9`
- `reviewStartedAtAsiaShanghai`: `2026-07-11T08:39:52.5204589+08:00`
- `reviewEndedAtAsiaShanghai`: `2026-07-11T08:42:49.5388480+08:00`
- `domainDimensions`: 覆盖、引用、自动/人工证据、回归、审计可复现
- `scoreVector`: `[12,10,10,10,8,6,4,8,8,8,8,7]`
- `total`: `99/100`
- `dimensionFloor`: `PASS`；逐维均不低于 `[10,8,8,8,7,5,4,7,7,7,7,7]`
- `P0`: `[]`
- `P1`: `[]`
- `P2`: `[]`
- `externalOrExecutionPending`: `[full CI execution, Windows/GPU/Spine controlled-runner evidence, clean-machine and AppContainer evidence]`
- `verdict`: `PASS`

章节证据：

- §1–2 明确 `VERIFIED/PARTIAL/UNVERIFIED`，并将合并、里程碑、Spine、RC 和商业发布分成五道门。
- §3.1–3.4 覆盖 unit/property/contract/E2E、NFR exact 注册和 registry/waiver/clock/commandKind 对抗矩阵。
- §3.5–5 将 Spine golden、合法合成素材、自动视觉指标与不可替代的人工审批分离。
- §6.1 固定 benchmark 环境、样本量、P95 算法、失败样本和最低硬件档，禁止择优重跑。
- §7–10 覆盖故障注入、脱敏诊断、不可变证据 Schema 和完成定义；CLI 缺失明确为 `SKIPPED_EXTERNAL_DEPENDENCY`。

第三人称结论：审阅者认为该文档已把功能、NFR、安全、Spine、性能、人工门和恢复证据组织为可执行测试体系。未给满分仅因为本次是计划快照审阅，完整 CI/硬件/外部工具证据尚未真实产生；文档没有把这种缺失写成 PASS。

### 2.4 16 — 工程规范、代码组织与协作

- `docId`: `F2S-DOC-ENG-001`
- `path`: `plan/reviews/snapshots/R3A-20260711-083804-FINAL2/plan/16-工程规范代码组织与协作.md`
- `inputSha256`: `2405d4d85522fab1739b5fa6b254cda85787d013ae20e2d3efb3fefbb5a744f4`
- `reviewStartedAtAsiaShanghai`: `2026-07-11T08:39:52.5204589+08:00`
- `reviewEndedAtAsiaShanghai`: `2026-07-11T08:42:49.5388480+08:00`
- `domainDimensions`: 干净 checkout 复现、版本锁定、诊断、回滚、离线安全交付
- `scoreVector`: `[12,9,10,10,8,6,4,8,8,8,8,7]`
- `total`: `98/100`
- `dimensionFloor`: `PASS`；逐维均不低于 `[10,8,8,8,7,5,4,7,7,7,7,7]`
- `P0`: `[]`
- `P1`: `[]`
- `P2`: `[]`
- `externalOrExecutionPending`: `[Git repository initialization, exact toolchain patch/lock evidence, executable CI/release pipeline evidence, signed clean-checkout reproduction]`
- `verdict`: `PASS`

章节证据：

- §2–5 给出单仓、依赖方向、模式采用/禁用理由及 TypeScript/Rust/Python 边界。
- §6–8 固定 ULID、tick、错误 registry、并发 CAS、tagged commandKind 和幂等重建规则。
- §9–11 将故障/安全负测、许可准入、作者独立复核和回滚意图纳入工程规范。
- §12.1–12.2 定义单一 release pipeline、逐阶段 hash 链和必须拒绝的治理/签名/许可/沙箱攻击向量。
- §13 的 DoD 要求验收、正负/故障测试、迁移、许可、追踪、回滚和真实证据同时存在。

第三人称结论：审阅者认为该文档足以指导绿地工程建立一致的代码、错误、CI 与发布纪律。其当前工作区和工具链仍明确为未初始化/`PLANNED`，所以复现与离线交付证据维度未取满分；这种诚实状态不会提前授权发布。

### 2.5 17 — 异常恢复、兼容与项目升级

- `docId`: `F2S-DOC-RECOVERY-001`
- `path`: `plan/reviews/snapshots/R3A-20260711-083804-FINAL2/plan/17-异常恢复兼容与项目升级.md`
- `inputSha256`: `ed50bc019952e14d3a3133005061101224cf314cac75af2782e17568df2a75e0`
- `reviewStartedAtAsiaShanghai`: `2026-07-11T08:39:52.5204589+08:00`
- `reviewEndedAtAsiaShanghai`: `2026-07-11T08:42:49.5388480+08:00`
- `domainDimensions`: 边界契约、模式取舍、状态/并发/版本、性能/恢复、替换/降级
- `scoreVector`: `[12,9,10,10,8,6,4,8,8,8,8,8]`
- `total`: `99/100`
- `dimensionFloor`: `PASS`；逐维均不低于 `[10,8,8,8,7,5,4,7,7,7,7,7]`
- `P0`: `[]`
- `P1`: `[]`
- `P2`: `[]`
- `externalOrExecutionPending`: `[Windows/NTFS durable-boundary evidence bound to F2S-TST-111, real Worker/GPU/Spine fault-injection evidence]`
- `verdict`: `PASS`

章节证据：

- §17.1–17.4 将 manifest/CAS/append 链、redb 重建、strict ULID、commandKind、enrollment/bundle 和全 epoch clock 作为恢复真值，扫描阶段只读且 fail closed。
- §17.4 对项目外 Worker sandbox 建立项目记录与目录的一对一对账，未知输出不会自动注册为成功。
- §17.5 对 import、Worker、远端、导出和 Spine CLI 分别规定中断、重试和不可伪造验证状态。
- §17.6–17.9 给出链式 schema 迁移、未知 major 只读、4.2.43 固定矩阵、应用回退和精确审批失效规则。
- §17.10–17.14 覆盖资源、双实例锁、Windows 环境、各 durable 边界故障注入和稳定验收。

第三人称结论：审阅者认为该文档对恢复真值、不可变资产、索引重建、跨 epoch 时钟、迁移和 Spine 兼容的边界完整且一致。未给满分仅因为真实 NTFS/进程/外部工具故障证据尚待执行；正文已把这一点保持为 `UNVERIFIED`，没有形成设计级 P0/P1。

## 3. 分片汇总

| 文档 | 向量 | 总分 | P0 | P1 | P2 | 结论 |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| 13 Export | `[12,9,9,10,8,6,4,8,8,8,8,8]` | 98 | 0 | 0 | 1 | PASS |
| 14 Security | `[12,9,10,10,8,6,4,8,8,7,8,8]` | 98 | 0 | 0 | 0 | PASS |
| 15 Quality | `[12,10,10,10,8,6,4,8,8,8,8,7]` | 99 | 0 | 0 | 0 | PASS |
| 16 Engineering | `[12,9,10,10,8,6,4,8,8,8,8,7]` | 98 | 0 | 0 | 0 | PASS |
| 17 Recovery | `[12,9,10,10,8,6,4,8,8,8,8,8]` | 99 | 0 | 0 | 0 | PASS |

本分片合计 `P0=0`、`P1=0`、`P2=1`，五份文档均达到总分与逐维 floor，R3a 分片结论为 `PASS`。该结论只评价上述冻结计划字节，不是代码、测试执行、Spine CLI、OS 沙箱、法务材料、原子计划、实施就绪或商业发布的批准。
