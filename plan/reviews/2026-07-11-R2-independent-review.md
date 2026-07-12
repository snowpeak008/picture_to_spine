---
evidence_id: F2S-REV-R2-001
review_round: R2
normalized_at: 2026-07-11T04:22:37+08:00
timezone: Asia/Shanghai
status: failed_legacy_review
source: independent agent mailbox reports
---

# R2 独立评审归一化记录

## 1. 审计声明

本文件保留第二轮只读评审的失败事实，不因后续整改覆盖。A、B、C、D批次均使用`F2S-DOC-SCORE-001`的60分通用+40分领域模型；任一P1即FAIL。审阅者以第三人称报告，不参与被审批次当轮整改。

R2执行时尚未建立Git仓库，也未在评审前归档25份输入文件的完整只读副本；B批次没有保存输入SHA，C/D仅在邮件中保存了缩略SHA。因此R2可以证明“发现过这些问题”，但不能作为最终计划门的可复现PASS证据。该流程缺陷本身记为`F2S-R2-AUDIT-001`；R3必须逐文件保存完整SHA-256、12维得分、审阅者、时间、问题和关闭证据，并且最终SHA与磁盘文件一致。

## 2. 审阅者与批次

| Batch | Files | Independent reviewer | Result |
| --- | --- | --- | --- |
| F2S-REVIEW-A-R2 | 00–05 | `/root/spine_version_license` | 6/6 FAIL |
| F2S-REVIEW-B-R2 | 06–12 | `/root/product_docs` | 7/7 FAIL |
| F2S-REVIEW-C-R2 | 13–18 | `/root/stack_arch` | 6/6 FAIL |
| F2S-REVIEW-D-R2 | 19–24 | `/root/spine_version_license` | 6/6 FAIL |

## 3. 逐文件分数

| File | R2 score | General/Domain（有记录时） | Result | Principal issue |
| --- | ---: | --- | --- | --- |
| 00 | 85 | 51/34 | FAIL | F2S-R2-A-OWNER-001 |
| 01 | 87 | 50/37 | FAIL | F2S-R2-A-SCOPE-001、F2S-R2-A-SANDBOX-001 |
| 02 | 87 | 47/40 | FAIL | F2S-R2-A-REQ-OWNER-001、F2S-R2-A-TLS-001 |
| 03 | 88 | 51/37 | FAIL | F2S-R2-A-TIME-001、F2S-R2-A-ASSET-001 |
| 04 | 86 | 51/35 | FAIL | F2S-R2-A-STATE-001、F2S-R2-A-WAIVER-001 |
| 05 | 92 | 55/37 | FAIL | F2S-R2-A-WAIVER-001 |
| 06 | 91 | 53/38 | FAIL | F2S-R2-B-PORT-001、F2S-R2-B-TESTOWNER-001 |
| 07 | 91 | 54/37 | FAIL | F2S-R2-B-ENV-001 |
| 08 | 85 | 50/35 | FAIL | F2S-R2-B-SNAPSHOT-001 |
| 09 | 82 | 48/34 | FAIL | F2S-R2-B-STATE-001、F2S-R2-B-JOB-001 |
| 10 | 79 | 46/33 | FAIL | F2S-R2-B-INPUT-001、F2S-R2-B-PROV-001 |
| 11 | 88 | 51/37 | FAIL | F2S-R2-B-PORT-001、F2S-R2-B-JOBSTORE-001 |
| 12 | 83 | 48/35 | FAIL | F2S-R2-B-HELLO-001、F2S-R2-B-SANDBOX-001 |
| 13 | 88 | 52/36 | FAIL | F2S-R2-C-CLI-001、F2S-R2-C-PSD-001 |
| 14 | 88 | 52/36 | FAIL | F2S-R2-C-SANDBOX-001、F2S-R2-C-MODEL-001 |
| 15 | 83 | 48/35 | FAIL | F2S-R2-C-QAOWNER-001、F2S-R2-C-PSD-001 |
| 16 | 82 | 48/34 | FAIL | F2S-R2-C-PATTERN-001、F2S-R2-C-ERRORREG-001 |
| 17 | 80 | 46/34 | FAIL | F2S-R2-C-RECOVERY-001、F2S-R2-C-JOB-001 |
| 18 | 83 | 49/34 | FAIL | F2S-R2-C-WAIVER-001、F2S-R2-C-LAUNCHER-001 |
| 19 | 83 | 50/33 | FAIL | F2S-R2-D-RISK-001 |
| 20 | 79 | 48/31 | FAIL | F2S-R2-D-DAG-001、F2S-R2-D-SLICE-001 |
| 21 | 92 | 57/35 | FAIL | F2S-R2-D-TRACE-001、F2S-R2-D-DEVPLAN-001 |
| 22 | 62 | 42/20 | FAIL | F2S-R2-AUDIT-001 |
| 23 | 69 | 47/22 | FAIL | F2S-R2-D-COMPLIANCE-001 |
| 24 | 74 | 45/29 | FAIL | F2S-R2-D-ADR-001、F2S-R2-D-TARGET-001 |

## 4. A批问题摘要（00–05）

| Issue | Severity | Reviewer finding |
| --- | --- | --- |
| F2S-R2-A-OWNER-001 | P1 | 审阅者认为治理、Gate、KPI、ENV/FIX、AST等namespace未形成单一owner闭环，frontmatter与正文声明不一致。 |
| F2S-R2-A-SCOPE-001 | P1 | 审阅者认为Scope/KPI缺少显式定义、公式、样本、TST/EVD，无法独立验收。 |
| F2S-R2-A-SANDBOX-001 | P1 | 审阅者认为“数据不离开项目目录”与%LOCALAPPDATA% Job sandbox冲突，未说明临时D2副本、ACL、期限和清理。 |
| F2S-R2-A-REQ-OWNER-001 | P1 | 审阅者认为ENV/FIX由02定义但未拥有，21又错误指向07。 |
| F2S-R2-A-TLS-001 | P1 | 审阅者认为显式dev mode跳过TLS与安全文档“证书错误不得忽略”冲突。 |
| F2S-R2-A-TIME-001 | P1 | 审阅者认为03的内部秒模型与08的i64 tick+有理timeBase冲突。 |
| F2S-R2-A-ASSET-001 | P1 | 审阅者认为AST和CONTENT-GATE未登记owner，动态SECONDARY仍用通配实体ID。 |
| F2S-R2-A-STATE-001 | P1 | 审阅者认为04/09的五维状态、Job枚举和spine_verified阶段发生漂移。 |
| F2S-R2-A-WAIVER-001 | P1 | 审阅者认为UX/UI对所有Warning提供接受风险，可能绕过P0/P1及安全/许可/数据门。 |

## 5. B批问题摘要（06–12）

| Issue | Severity | Reviewer finding |
| --- | --- | --- |
| F2S-R2-B-PORT-001 | P1 | 06与11定义两个同名不同签名ProjectRepository；InferenceProvider与ComputeProvider也缺适配映射。 |
| F2S-R2-B-TESTOWNER-001 | P1 | 领域TST frontmatter与21“全部由15拥有”规则冲突。 |
| F2S-R2-B-ENV-001 | P1 | 工具命令证据状态、WebView2缺失测试和入口权威边界不完整。 |
| F2S-R2-B-SNAPSHOT-001 | P1 | RenderSnapshot/Patch缺projectId、snapshotId、streamEpoch，重连或切项目可能串流。 |
| F2S-R2-B-STATE-001 | P1 | 09把stale/blocked/failed/unverified混回对象生命周期和项目阶段。 |
| F2S-R2-B-JOB-001 | P1 | Job succeeded究竟是否已经RegisterCandidate不唯一，取消和revision冲突边界无法实现。 |
| F2S-R2-B-INPUT-001 | P1 | 10错误支持PSD/PSB输入，与P0 PNG/JPEG/WebP及PSD只输出冲突。 |
| F2S-R2-B-PROV-001 | P1 | GenerationProvenance硬门、unknown_external和Stage取消/幂等/checkpoint契约缺失。 |
| F2S-R2-B-JOBSTORE-001 | P1 | redb/OperationJournal未给JobExecutionRecord权威schema和重建算法。 |
| F2S-R2-B-HELLO-001 | P1 | hello示例缺envelope必填字段，seq/session规则和资源上限不可断言。 |
| F2S-R2-B-SANDBOX-001 | P1 | Worker隔离仍是多种候选/策略约定，没有唯一商业Profile和恶意探针退出条件。 |

## 6. C批问题摘要（13–18）

| Issue | Severity | Reviewer finding |
| --- | --- | --- |
| F2S-R2-C-CLI-001 | P1 | probe与转换分进程且普通转换未钉扎4.2.43，存在active patch TOCTOU。 |
| F2S-R2-C-PSD-001 | P1 | 最小PSD的层级/visibility/alpha/origin/pivot/reopen没有字段级Profile和golden。 |
| F2S-R2-C-SANDBOX-001 | P1 | 12/14对AppContainer、restricted token、ACL、Job Object的发布必需组合不唯一，测试门缺失。 |
| F2S-R2-C-MODEL-001 | P1 | Model Pack未禁止Pickle/torch.load等可执行反序列化权重。 |
| F2S-R2-C-QAOWNER-001 | P1 | QA gates无canonical owner，license门复制不完整policy，OS sandbox和PSD断言缺失。 |
| F2S-R2-C-PATTERN-001 | P1 | 16越权定义ADR-PAT，而ADR唯一owner是24。 |
| F2S-R2-C-ERRORREG-001 | P1 | error registry交付、重复/未登记CI负测与exact DEV/EVD未绑定。 |
| F2S-R2-C-RECOVERY-001 | P1 | 17复制过时提交算法，未扫描项目外sandbox，也未按11真值恢复。 |
| F2S-R2-C-JOB-001 | P1 | Worker崩溃/远端断线创造私有Job状态，OOM可能静默改配置。 |
| F2S-R2-C-WAIVER-001 | P1 | 18仍允许非核心P1 waiver，与计划P0/P1清零硬门冲突。 |
| F2S-R2-C-LAUNCHER-001 | P1 | batch echo路径会重新解析&等元字符，portable/release manifest签名根和反回滚也不完整。 |

## 7. D批问题摘要（19–24）

| Issue | Severity | Reviewer finding |
| --- | --- | --- |
| F2S-R2-D-RISK-001 | P1 | 风险表把未存在的R2 hash标MITIGATED，PSD与P0冲突，缺SEC-005风险，引用不存在的devplan/97。 |
| F2S-R2-D-DAG-001 | P1 | M06、M04/M05/M07的文字并行关系与表/DAG依赖互相矛盾。 |
| F2S-R2-D-SLICE-001 | P1 | 声称M00–M03后已有分层/Rig/预览/Spine JSON，实际能力属于M04/M05/M07/M08。 |
| F2S-R2-D-TRACE-001 | P1 | 第二轮早期存在71个领域测试无反向边；后续虽补至133/133，NFR/DEV的AC审计规则仍自相矛盾。 |
| F2S-R2-D-DEVPLAN-001 | P1 | reserved矩阵不等于任务卡；R2时plan/devplan为0文件，原子门不得通过。 |
| F2S-R2-AUDIT-001 | P1 | R1/R2缺完整12维、输入快照、raw report/identity/time，旧hash不能独立复现。 |
| F2S-R2-D-COMPLIANCE-001 | P1 | 23仍是pending框架，使用devplan/97/98/99，PASS没有命令输出/hash。 |
| F2S-R2-D-ADR-001 | P1 | 重大Sandbox/Waiver/Time/Job/PSD/CLI决策未进入24，EVD命名不合规。 |
| F2S-R2-D-TARGET-001 | P1 | 用户没有明确选择五个集成目标之一，24却把“仅Spine Editor”写为accepted。 |

## 8. 已知输入SHA记录缺口

A批邮件保存了完整输入SHA：00 `681274bfa55c0c79a2bebdaf86b23d93c7ec055b6bff3cd78b80fb455ac3b565`；01 `6f6191e4197487dde8dc4219031bd288a10f318e79b39d60f9f359c9bd83812f`；02 `2b5f86133b0aaccf4b5bd1b588ff575263f3dc8e6987b73e9c863ed314ae3442`；03 `af5e109e53a29db322bb6b53c067e6f0398bfb73ed1eb466463c32eec7825790`；04 `fec6ceadc38f1877913cf78b9dd8858aed1f3836f6380c70c9563a7897594923`；05 `1b8fe67f621d131ac3f760fc836a9cc878343a93d220f69eaa5659d3fe708d00`。

B批未归档输入SHA；C/D邮件只保存缩略SHA，不足以独立校验。R3不得沿用这个缺口。
