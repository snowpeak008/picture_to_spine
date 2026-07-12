---
doc_id: F2S-DOC-DEVPLAN-M00-001
revision: 1.0
status: draft
canonical_for: [F2S-DEVPLAN-M00-CARDS-001, F2S-WU-M00]
depends_on: [F2S-DOC-DEVPLAN-INDEX-001]
review_score_ref: F2S-SCORE-DEVPLAN-M00-001-D0
last_verified: 2026-07-11
---

# M00 决策与可行性 Spike 原子计划

本文件只定义实施期工作，不宣称任何 Spike 已执行。能力初态均为 UNVERIFIED；需要用户合法商业工具、干净 Windows 11、特定 GPU 或许可证意见的能力另标 EXTERNAL。所有命令均通过 00 号文件冻结的顶层合同进入，计划阶段不安装依赖、不生成伪 PASS 证据。

## F2S-DEV-M00-001 — 决策与精确工具链基线

#### 1. 任务头

- 目标：在同一份可机读探测结果中冻结 Node、npm、Rust/Cargo、Python、uv、MSVC、WebView2 与 Windows SDK 的精确 patch、来源和二进制 hash，并确认 V1 首个集成目标为 Spine Editor 4.2.43。
- 非目标：不安装工具链，不把本机观察值写成 clean-VM VERIFIED，不实现 M01 锁文件或产品代码。
- 预计：2d。

#### 2. 追踪

- FR/NFR：F2S-NFR-COMPAT-001、F2S-NFR-MAINT-001。
- ADR：F2S-ADR-PLAT-001、F2S-ADR-STACK-001、F2S-ADR-TARGET-001。
- 风险：工具自动升级、PATH 命中错误副本、WebView2/MSVC 缺失、Spine patch 漂移。
- 上游 DEV：F2S-DOC-DEVPLAN-INDEX-001；下游 DEV：F2S-DEV-M01-001、F2S-DEV-M01-002、F2S-DEV-M00-005。
- 携带项：P0/P1 无；P2 F2S-R3A-B07-P2-001，必须保留精确 patch、可执行文件 SHA-256、安装来源和 clean-VM 复现。

#### 3. 输入

- artifact：00 号文件记录的 OBSERVED_LOCAL 候选 Node 24.15.0、npm 11.12.1、Rust/Cargo 1.96.0、Python 3.12.4、uv 0.11.8，状态 UNVERIFIED_CLEAN_VM。
- config：Windows 11 x64 干净 runner 清单；MSVC、Windows SDK、WebView2 固定来源待探测。
- schema：F2S-TOOLCHAIN-PROBE-001 v1（本任务定义）；fixture：无。

#### 4. 输出

- tools/spikes/m00/toolchain-probe.ps1：只读探测器，owner F2S-DEV-M00-001，可再生。
- tools/spikes/m00/toolchain-probe.schema.json：探测证据 schema，owner 本任务，不可由运行结果改写。
- docs/decisions/F2S-TOOLCHAIN-BASELINE-001.md：接受值、来源、hash 与偏差裁决，owner 本任务。
- docs/decisions/F2S-INTEGRATION-TARGET-001.md：Spine Editor 4.2.43 单一目标边界，owner 本任务。

#### 5. 设计边界

- 归属：工具探测属于 Adapter/工程治理；决策文档属于架构基线，不进入 Domain。
- 使用：Probe Adapter、Decision Record、fail-closed validation。
- 拒绝：依赖 PATH 文字版本即通过、latest 标签、自动下载安装、将本机观察等同干净机验证。

#### 6. work units

##### F2S-WU-M00-001-01 — 可复现工具链只读探测器

- output: tools/spikes/m00/toolchain-probe.ps1
- reads: F2S-ENV-WIN-001、F2S-ADR-STACK-001、F2S-DOC-DEVPLAN-INDEX-001
- writes: tools/spikes/m00/toolchain-probe.ps1；tools/spikes/m00/toolchain-probe.schema.json
- steps: 1) 固定字段和排序；2) 用显式可执行文件路径探测版本；3) 计算可执行文件 SHA-256；4) 记录安装来源、退出码和 UTC；5) 对缺失项输出 UNVERIFIED 而非成功；6) 以 schema 校验结果。
- command: npm run bootstrap:check
- tests: F2S-TST-E2E-COMPAT-001；向量为正常安装、PATH 双副本、缺 MSVC、hash 不符。
- evidence: evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-01/probe.json 与 probe.log
- dependsOn: F2S-DOC-DEVPLAN-INDEX-001
- parallelSafety: isolated
- rollback: 删除本 WU 两个新增文件；不修改机器工具链或注册表。
- estimate: 0.5d

##### F2S-WU-M00-001-02 — clean-VM 精确基线裁决

- output: docs/decisions/F2S-TOOLCHAIN-BASELINE-001.md
- reads: F2S-WU-M00-001-01、tools/spikes/m00/toolchain-probe.schema.json、evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-02/clean-vm-a/probe.json、evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-02/clean-vm-b/probe.json、F2S-DOC-DEVPLAN-INDEX-001
- writes: docs/decisions/F2S-TOOLCHAIN-BASELINE-001.md
- steps: 1) 比较两个干净机结果；2) 固定每个工具 patch 和分发来源；3) 钉住二进制 hash 或签名身份；4) 记录 MSVC/WebView2/SDK；5) 对差异给出拒绝或重跑；6) 未取得双 runner 时保持 UNVERIFIED。
- command: npm run bootstrap:check
- tests: F2S-TST-E2E-COMPAT-001、F2S-TST-E2E-MAINT-001；边界为精确相等、patch 差 1、hash 差 1 bit、第二 runner 缺失。
- evidence: evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-02/baseline-diff.json 与 clean-vm.log
- dependsOn: F2S-WU-M00-001-01
- parallelSafety: sequential
- rollback: 恢复上一版决策文档；证据只追加，不覆盖旧运行。
- estimate: 1d

##### F2S-WU-M00-001-03 — Spine Editor 4.2.43 目标边界

- output: docs/decisions/F2S-INTEGRATION-TARGET-001.md
- reads: F2S-ADR-TARGET-001、F2S-ADR-SPN-001、F2S-ADR-CLI-001、F2S-OQ-SPN-001
- writes: docs/decisions/F2S-INTEGRATION-TARGET-001.md
- steps: 1) 固定 Editor 4.2.43；2) 列出内置开放输出；3) 列出用户 CLI 专有输出；4) 标明不捆绑 Editor/Runtime；5) 定义 EXTERNAL 往返证据；6) 定义变更需新 ADR。
- command: npm run test:spine
- tests: F2S-TST-E2E-007；向量为 4.2.43、4.2.42、4.2.44、无合法 Editor。
- evidence: evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-03/target-decision-check.json
- dependsOn: F2S-WU-M00-001-02
- parallelSafety: isolated
- rollback: 恢复上一接受版本；禁止静默改 patch。
- estimate: 0.5d

#### 7. 正向验收

- F2S-TST-E2E-COMPAT-001：两个干净 Windows 11 runner 输入相同安装来源，预期所有精确 patch、签名身份/hash、退出码一致，能力才可转 VERIFIED。
- F2S-TST-E2E-MAINT-001：从探测 JSON 生成 M01 消费清单，预期字段完整且零 latest/范围版本。
- F2S-TST-E2E-007：目标清单只能得到 Spine Editor 4.2.43；开放候选输出与 CLI-owned 输出严格分列。

#### 8. 负向与故障验收

- 缺工具、PATH 双副本、hash 不符、脚本非零退出、离线 runner 或第二 runner 缺失均不得写 VERIFIED。
- 检测到 4.2.42/4.2.44、捆绑 Spine Runtime 或把 atlas/spine/skel 归为内置 writer 时失败。

#### 9. 证据

- F2S-EVD-M00-001：evidence/M00/F2S-DEV-M00-001/evidence.json；同号任务证据索引。
- evidence.json 必填 taskId、status、command、exitCode、startedAtUtc、endedAtUtc、runner、toolVersions、inputHashes、outputHashes、logRefs、reportRefs、capabilityState、externalBlockers、previousEvidenceRef；初态 status=NOT_RUN、capabilityState=UNVERIFIED。

#### 10. 回滚

- 代码/配置：移除探测脚本和未接受决策，恢复上一 hash 锁；不回退机器安装。
- schema/项目数据：无项目 schema 或用户数据变更。
- 外部副作用：runner 只读；失败证据保留并以 previousEvidenceRef 串联。

#### 11. 完成定义

- 统一门禁：npm run format:check、npm run lint、npm run typecheck、npm test、npm run build:core 均有结构化结果；文档、追踪、许可审计和用户可见错误处理完成。

- npm run format:check、npm run lint、npm run typecheck、npm run bootstrap:check 全部有结构化结果；文档、追踪、hash 和来源齐全。
- 许可扫描确认探测脚本无禁止依赖；中文错误能指出缺失工具及修复路径。

#### 12. 退出状态

- 任务 done 只表示基线和探测合同完成；clean-VM 证据不足时能力仍 UNVERIFIED。
- Spine Editor 合法副本属于 EXTERNAL；缺本任务同号证据时阻断 M01 锁版和任何兼容性发布声明。

## F2S-DEV-M00-002 — 许可证与外部系统清单

#### 1. 任务头

- 目标：建立发布依赖、构建工具、模型/权重、系统运行时和用户商业工具的可机读清单，并按 F2S-LIC-POLICY-001 fail closed。
- 非目标：不提供法律意见，不把 Windows、CUDA、Spine 或用户 CLI 伪装成开源依赖，不批准未知许可证。
- 预计：1.5d。

#### 2. 追踪

- FR/NFR：F2S-NFR-LIC-001、F2S-NFR-LIC-002。
- ADR：F2S-ADR-LIC-001、F2S-ADR-CLI-001、F2S-ADR-SPN-003。
- 风险：传递依赖漏审、字体/图标/权重来源缺失、系统组件错误打包。
- 上游 DEV：F2S-DEV-M00-001；下游 DEV：F2S-DEV-M00-003、F2S-DEV-M01-002、F2S-DEV-M09-004。
- 携带项：P0/P1 无；P2 无。

#### 3. 输入

- policy：F2S-LIC-POLICY-001 当前宽松许可 allowlist；未知项一律拒绝。
- artifact：M00 工具链来源清单、F2S-OQ-LGL-001。
- schema：F2S-SUPPLY-INVENTORY-001 v1；fixture：一组 MIT、Apache-2.0、BSD-3-Clause、unknown、GPL/AGPL/NC 合成条目。

#### 4. 输出

- docs/compliance/F2S-SUPPLY-INVENTORY-001.json：依赖与资产登记，owner F2S-DEV-M00-002，不可手工删历史条目。
- docs/compliance/F2S-EXTERNAL-SYSTEMS-001.json：系统/硬件/用户工具边界，owner 本任务。
- docs/compliance/F2S-LICENSE-REVIEW-PLAYBOOK-001.md：审计与升级流程，owner 本任务。
- tools/compliance/license-inventory-check.mjs：离线校验器，owner 本任务，可再生。

#### 5. 设计边界

- 归属：合规 Policy 与 Inventory；校验器为工程 Adapter，不进入业务 Domain。
- 使用：Policy Object、Registry、allowlist、不可变审计条目。
- 拒绝：名称猜许可证、网络实时查询作为唯一真值、waiver 绕过未知/禁止项、混合 Core 与 Worker Pack。

#### 6. work units

##### F2S-WU-M00-002-01 — 五类供应项登记模型

- output: docs/compliance/F2S-SUPPLY-INVENTORY-001.json
- reads: F2S-LIC-POLICY-001、F2S-ADR-LIC-001、F2S-ADR-CLI-001、F2S-DEV-M00-001、docs/decisions/F2S-TOOLCHAIN-BASELINE-001.md
- writes: docs/compliance/F2S-SUPPLY-INVENTORY-001.json；docs/compliance/F2S-EXTERNAL-SYSTEMS-001.json
- steps: 1) 定义发布依赖/构建工具/资产权重/系统运行时/用户商业工具；2) 记录版本、来源、hash、许可证据；3) 标记是否进入包；4) 分离 Core/Worker Pack；5) 未知项置 blocked；6) 固定排序。
- command: npm run release:verify
- tests: F2S-TST-E2E-LIC-001；向量覆盖五类条目和遗漏 licenseEvidence。
- evidence: evidence/M00/F2S-DEV-M00-002/F2S-WU-M00-002-01/inventory-report.json
- dependsOn: F2S-DEV-M00-001
- parallelSafety: isolated
- rollback: 恢复上一 inventory revision；历史证据不删除。
- estimate: 0.5d

##### F2S-WU-M00-002-02 — 离线许可证 fail-closed 校验

- output: tools/compliance/license-inventory-check.mjs
- reads: docs/compliance/F2S-SUPPLY-INVENTORY-001.json；F2S-LIC-POLICY-001
- writes: tools/compliance/license-inventory-check.mjs
- steps: 1) 读取锁文件和登记表；2) 规范化 SPDX 表达式但不猜测；3) 展开传递依赖；4) 检查资产与权重；5) 对 unknown/无证据/禁止项非零退出；6) 输出稳定 JSON。
- command: npm run release:verify
- tests: F2S-TST-E2E-LIC-001、F2S-TST-E2E-LIC-002；MIT/Apache/BSD 通过，GPL/LGPL/AGPL/MPL/SSPL/NC/Research-only/unknown 拒绝。
- evidence: evidence/M00/F2S-DEV-M00-002/F2S-WU-M00-002-02/license-check.json
- dependsOn: F2S-WU-M00-002-01
- parallelSafety: isolated
- rollback: 删除校验器并恢复上一已审查版本；不得把拒绝项改为 warning。
- estimate: 0.5d

##### F2S-WU-M00-002-03 — 合规审阅与升级剧本

- output: docs/compliance/F2S-LICENSE-REVIEW-PLAYBOOK-001.md
- reads: docs/compliance/F2S-SUPPLY-INVENTORY-001.json、docs/compliance/F2S-EXTERNAL-SYSTEMS-001.json、F2S-OQ-LGL-001、F2S-OQ-SPN-001
- writes: docs/compliance/F2S-LICENSE-REVIEW-PLAYBOOK-001.md
- steps: 1) 定义新增项申请；2) 定义法务证据字段；3) 定义 Core/Worker 物理移除；4) 定义 Spine/CUDA/Windows 外部条件；5) 定义失败发布门；6) 记录需新 policy 的变更。
- command: npm run release:verify
- tests: F2S-TST-E2E-LIC-002；缺法务意见、错分类、禁止项试图 waiver 均阻断。
- evidence: evidence/M00/F2S-DEV-M00-002/F2S-WU-M00-002-03/playbook-review.json
- dependsOn: F2S-WU-M00-002-02
- parallelSafety: isolated
- rollback: 恢复上一 playbook；不改变已登记的外部责任。
- estimate: 0.5d

#### 7. 正向验收

- F2S-TST-E2E-LIC-001：许可明确且在 allowlist 的锁定依赖，预期报告 PASS 并列出传递闭包、来源和 package inclusion。
- F2S-TST-E2E-LIC-002：Windows/WebView2/CUDA/Spine 被精确分类为外部系统或用户工具，发布清单不包含其二进制。

#### 8. 负向与故障验收

- unknown、无许可证、证据链接/hash 缺失或禁止 SPDX 一律非零退出；P2 waiver 不能改变结果。
- SBOM/lock/inventory 任一漂移、网络不可用、登记表损坏时 fail closed，Core 构建可继续但 release verify 失败。

#### 9. 证据

- F2S-EVD-M00-002：evidence/M00/F2S-DEV-M00-002/evidence.json；同号任务证据索引。
- evidence.json 必填 taskId、status、command、exitCode、startedAtUtc、endedAtUtc、runner、toolVersions、inputHashes、outputHashes、logRefs、reportRefs、capabilityState、externalBlockers、previousEvidenceRef；reportRefs 另指向 inventory、license report 和 package inclusion diff；初态 NOT_RUN/UNVERIFIED。

#### 10. 回滚

- 恢复上一接受 inventory/policy 适配器；任何新增未审项从发布图物理移除。
- 无项目数据迁移；不撤销历史合规证据；无外部网络写入。

#### 11. 完成定义

- 统一门禁：npm run format:check、npm run lint、npm run typecheck、npm test、npm run build:core 均有结构化结果；文档、追踪、许可审计和用户可见错误处理完成。

- format/lint/typecheck/release:verify 均有结果；清单、审计流程、测试向量和追踪齐全。
- 所有脚本及测试 fixture 自身也通过许可审计，用户可见错误给出包名、版本、分类与阻断理由。

#### 12. 退出状态

- done 不代表法律意见 VERIFIED；F2S-OQ-LGL-001 仍为 EXTERNAL。
- 任一 unknown/禁止项、Spine 官方组件误入包或清单漂移都阻断商业发布。

## F2S-DEV-M00-003 — 合法合成fixture基线

#### 1. 任务头

- 目标：建立完全自制、来源可审计的二次元类人横版合成角色 fixture，覆盖图片导入、母版、分层、Rig、十动作和报告链的最小测试输入。
- 非目标：不复制用户图片，不宣称合成 fixture 代表真实目标域成功率，不生成最终商业美术。
- 预计：1.5d。

#### 2. 追踪

- FR/NFR：F2S-NFR-MAINT-001、F2S-FR-REPORT-001。
- ADR：F2S-ADR-ACTION-001、F2S-ADR-WEAPON-001、F2S-ADR-AST-001。
- 风险：来源不清、fixture 随工具漂移、只覆盖成功样本。
- 上游 DEV：F2S-DEV-M00-002；下游 DEV：F2S-DEV-M03-002、F2S-DEV-M04-001、F2S-DEV-M11-001。
- 携带项：P0/P1 无；P2 无；保留 F2S-OQ-DATASET-001 的真实目标域外部边界。

#### 3. 输入

- policy：M00-002 合规 playbook；动作 registry 固定十个 clip。
- fixture source：团队原创矢量几何、无品牌单手训练剑，仅作为 fixture。
- schema：F2S-FIXTURE-MANIFEST-001 v1；hash 状态在生成后才可 VERIFIED。

#### 4. 输出

- fixtures/m00/synthetic-character/source.svg：原创源，owner F2S-DEV-M00-003。
- fixtures/m00/synthetic-character/master.png：确定性渲染，owner 本任务，可再生。
- fixtures/m00/synthetic-character/action-keyframes.png：十动作关键姿势拼图，owner 本任务，可再生。
- fixtures/m00/synthetic-character/manifest.json、LICENSES.json、hashes.sha256：来源、生成参数与 hash，owner 本任务。
- docs/testing/F2S-FIXTURE-POLICY-001.md：允许用途和真实域限制，owner 本任务。

#### 5. 设计边界

- 归属：测试 fixture 与 provenance，不进入产品资产。
- 使用：Test Data Builder、Golden Master、content-addressed provenance。
- 拒绝：抓取网络图、使用用户图回填仓库、手改渲染 PNG 后不更新源/hash、以 fixture 指标替代真人验收。

#### 6. work units

##### F2S-WU-M00-003-01 — 原创角色源与可审计清单

- output: fixtures/m00/synthetic-character/manifest.json
- reads: F2S-ADR-ACTION-001、F2S-ADR-WEAPON-001、F2S-DEV-M00-002、docs/compliance/F2S-LICENSE-REVIEW-PLAYBOOK-001.md
- writes: fixtures/m00/synthetic-character/source.svg；fixtures/m00/synthetic-character/manifest.json；fixtures/m00/synthetic-character/LICENSES.json；docs/testing/F2S-FIXTURE-POLICY-001.md
- steps: 1) 用基础几何绘制侧视类人；2) 添加无品牌训练剑；3) 定义十动作关键姿势参数；4) 写作者与许可声明；5) 记录生成器版本；6) 明示仅测试用途。
- command: npm test
- tests: F2S-TST-E2E-MAINT-001；manifest 必含 source、license、generator、actionIds、expected hashes。
- evidence: evidence/M00/F2S-DEV-M00-003/F2S-WU-M00-003-01/provenance-check.json
- dependsOn: F2S-DEV-M00-002
- parallelSafety: isolated
- rollback: 删除未接受 fixture revision；保留上一 hash 对应源和许可。
- estimate: 1d

##### F2S-WU-M00-003-02 — 确定性渲染与 golden hash

- output: fixtures/m00/synthetic-character/hashes.sha256
- reads: F2S-WU-M00-003-01、fixtures/m00/synthetic-character/source.svg、fixtures/m00/synthetic-character/manifest.json、F2S-DEV-M00-001
- writes: fixtures/m00/synthetic-character/master.png；fixtures/m00/synthetic-character/action-keyframes.png；fixtures/m00/synthetic-character/hashes.sha256
- steps: 1) 离线渲染固定画布；2) 禁止元数据时间戳；3) 生成十动作拼图；4) 重跑两次逐 bytes 比较；5) 写 SHA-256；6) 校验透明度和动作数量。
- command: npm test
- tests: F2S-TST-E2E-MAINT-001、F2S-TST-E2E-005；两次 bytes 相同、恰十动作、源变 1 bit 必须 hash 改变。
- evidence: evidence/M00/F2S-DEV-M00-003/F2S-WU-M00-003-02/render-hashes.json 与 contact-sheet.png
- dependsOn: F2S-WU-M00-003-01
- parallelSafety: sequential
- rollback: 从 source.svg 重生；禁止保留 hash 不匹配 PNG。
- estimate: 1d

#### 7. 正向验收

- F2S-TST-E2E-MAINT-001：相同工具链连续两次渲染，master、keyframes 逐 bytes 相同，manifest/hash 全匹配。
- F2S-TST-E2E-005：关键姿势拼图含十个唯一 actionId，能作为后续人工审批流程输入但默认未批准。

#### 8. 负向与故障验收

- 缺许可、作者、生成器版本、源文件或 hash 时 fixture 不可进入测试；发现网络素材立即隔离。
- 工具漂移、PNG 元数据不确定、动作缺失/重复、用户素材误入时测试失败；不得修补 expected hash 掩盖差异。

#### 9. 证据

- F2S-EVD-M00-003：evidence/M00/F2S-DEV-M00-003/evidence.json；同号任务证据索引。
- evidence.json 必填 taskId、status、command、exitCode、startedAtUtc、endedAtUtc、runner、toolVersions、inputHashes、outputHashes、logRefs、reportRefs、capabilityState、externalBlockers、previousEvidenceRef；inputHashes 指向 source/manifest，outputHashes 指向两 PNG；初态 NOT_RUN/UNVERIFIED。

#### 10. 回滚

- 以 source+generator 重生可再生 PNG；许可或 provenance 有疑义时整套 fixture 从仓库与发布测试包移除。
- 无项目 schema、用户配置或外部副作用。

#### 11. 完成定义

- 统一门禁：npm run format:check、npm run lint、npm run typecheck、npm test、npm run build:core 均有结构化结果；文档、追踪、许可审计和用户可见错误处理完成。

- format/lint/typecheck/test 通过；来源、许可、十动作、hash、真实域限制和追踪均齐全。
- 失败信息指出具体缺失字段或不一致文件，不输出用户数据。

#### 12. 退出状态

- done 仅验证合成 fixture 基线；真实授权目标域素材仍 EXTERNAL。
- F2S-OQ-DATASET-001 未关闭前，不得发布真实目标域成功率、质量百分比或重度动作游戏生产承诺。

## F2S-DEV-M00-004 — 8GB GPU与AppContainer可行性Spike

#### 1. 任务头

- 目标：在 Windows 11、8GB NVIDIA GPU 档验证可选 AI Worker 的显存降级矩阵，并验证商业 D2 Worker 同时满足 AppContainer、无网络、专用 ACL、Job Object 与恶意探针。
- 非目标：不安装 Python/CUDA/模型，不把普通用户进程、防火墙或仅 ACL 当等价沙箱，不要求 GPU 才能使用 Core。
- 预计：3d。

#### 2. 追踪

- FR/NFR：F2S-NFR-PERF-006、F2S-FR-COMP-002、F2S-NFR-SEC-005。
- ADR：F2S-ADR-GPU-001、F2S-ADR-SANDBOX-001。
- 风险：8GB OOM、驱动/CUDA/Python 组合不兼容、AppContainer breakaway、文件或网络越权。
- 上游 DEV：F2S-DEV-M00-001、F2S-DEV-M00-002；下游 DEV：F2S-DEV-M09-001。
- 携带项：P0/P1 无；P2 F2S-R3A-B12-P2-001，必须覆盖 AppContainer+Python/CUDA/GPU 组合探针。

#### 3. 输入

- config：F2S-ENV-WIN-001；候选 8GB GPU、驱动、Python、CUDA 组合只来自用户授权测试机。
- fixture：M00-003 合成角色；恶意探针为自制网络、路径、句柄、子进程和 breakaway 请求。
- schema：F2S-SANDBOX-PROBE-001 v1、F2S-GPU-PROFILE-PROBE-001 v1。

#### 4. 输出

- tools/spikes/m00/appcontainer-probe.ps1：隔离组合探针，owner F2S-DEV-M00-004。
- tools/spikes/m00/gpu-profile-probe.ps1：显存与降级探针，owner 本任务。
- tools/spikes/m00/spike-result.schema.json：统一结果 schema，owner 本任务。
- docs/spikes/F2S-M00-8GB-APPCONTAINER-001.md：矩阵、失败模式和后续裁决，owner 本任务。
- 再生性：两个探针和 schema 是受版本控制源；报告可由同号 evidence 重生但不得覆盖历史裁决。

#### 5. 设计边界

- 归属：安全/性能 Spike Adapter；产品 Core 只消费能力结果。
- 使用：Capability Probe、Bulkhead、fail-closed feature toggle、Strategy matrix。
- 拒绝：policy_only 降级、自动联网装驱动、GPU 失败拖垮 Core、记录凭据或用户图。

#### 6. work units

##### F2S-WU-M00-004-01 — AppContainer 五控制组合探针

- output: tools/spikes/m00/appcontainer-probe.ps1
- reads: F2S-ADR-SANDBOX-001、F2S-SEC-SANDBOX-001、F2S-DEV-M00-002、docs/compliance/F2S-EXTERNAL-SYSTEMS-001.json
- writes: tools/spikes/m00/appcontainer-probe.ps1；tools/spikes/m00/spike-result.schema.json
- steps: 1) 创建 restricted token；2) 配置无 network capability；3) 建专用 ACL Job root；4) 施加 Job Object 限制；5) 执行网络/路径/句柄/breakaway 探针；6) 任一控制缺失返回 FAILED。
- command: npm run test:integration
- tests: F2S-TST-RGPU-005、F2S-TST-E2E-SEC-005；五控制全在、逐一移除、子进程逃逸和非 Job root 写入。
- evidence: evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-01/sandbox-probe.json 与 os-events.log
- dependsOn: F2S-DEV-M00-002
- parallelSafety: shared-lock:windows-appcontainer-profile
- rollback: 删除临时 profile/ACL/Job root；确认无残留进程、网络规则或用户数据。
- estimate: 1d

##### F2S-WU-M00-004-02 — 8GB GPU 与 Python/CUDA 降级矩阵

- output: tools/spikes/m00/gpu-profile-probe.ps1
- reads: F2S-DEV-M00-001、docs/decisions/F2S-TOOLCHAIN-BASELINE-001.md、F2S-DEV-M00-003、fixtures/m00/synthetic-character/manifest.json、F2S-ENV-WIN-001
- writes: tools/spikes/m00/gpu-profile-probe.ps1
- steps: 1) 采集 GPU/驱动/Python/CUDA 精确身份；2) 运行 512/1024/2048 档合成负载；3) 记录峰值 VRAM/RAM/时延；4) 注入 OOM/取消；5) 验证回退 CPU/禁用 Worker；6) 清理缓存且不持久化素材。
- command: npm run test:integration
- tests: F2S-TST-E2E-PERF-006；向量为 8GB、低于 8GB、无 CUDA、错误 driver、OOM、取消。
- evidence: evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-02/gpu-matrix.json 与 resource.csv
- dependsOn: F2S-WU-M00-004-01、F2S-DEV-M00-003
- parallelSafety: shared-lock:gpu0
- rollback: 终止探针 Job Object，删除专用缓存；不改驱动、Python 或 CUDA 安装。
- estimate: 1d

##### F2S-WU-M00-004-03 — 组合裁决与物理移除规则

- output: docs/spikes/F2S-M00-8GB-APPCONTAINER-001.md
- reads: F2S-WU-M00-004-01、F2S-WU-M00-004-02、evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-01/sandbox-probe.json、evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-02/gpu-matrix.json、F2S-R3A-B12-P2-001、F2S-LIC-POLICY-001
- writes: docs/spikes/F2S-M00-8GB-APPCONTAINER-001.md
- steps: 1) 交叉连接 sandbox 与 GPU 结果；2) 定义 VERIFIED 组合；3) 定义 UNVERIFIED/FAILED；4) 定义 Worker Pack 物理移除；5) 定义 Core 无 GPU 路径；6) 列出 M09 复验入口。
- command: npm run release:verify
- tests: F2S-TST-RGPU-005、F2S-TST-E2E-PERF-006；任一沙箱控制失败时即使 GPU 成功也不得启用 Worker。
- evidence: evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-03/decision-matrix.json
- dependsOn: F2S-WU-M00-004-01、F2S-WU-M00-004-02
- parallelSafety: sequential
- rollback: 将 Worker capability 退回 UNVERIFIED 并从 release graph 移除；Core 不回滚。
- estimate: 1d

#### 7. 正向验收

- F2S-TST-RGPU-005：五项沙箱控制同时有效且恶意探针全部被拒，预期 windows-appcontainer-v1 可进入后续复验。
- F2S-TST-E2E-PERF-006：8GB 档完成规定负载且峰值不越预算，OOM/取消后 Core 仍可操作并输出结构化结果。

#### 8. 负向与故障验收

- 任一 AppContainer 控制缺失、网络可达、越 Job root、句柄继承或 breakaway 成功，Provider 状态 FAILED 且 Worker Pack 必须物理移除。
- Python/CUDA/driver 不符、OOM、掉卡、杀进程或清理失败不得崩溃 Core；不得自动下载安装或上传 fixture。

#### 9. 证据

- F2S-EVD-M00-004：evidence/M00/F2S-DEV-M00-004/evidence.json；同号任务证据索引。
- evidence.json 必填 taskId、status、command、exitCode、startedAtUtc、endedAtUtc、runner、toolVersions、inputHashes、outputHashes、logRefs、reportRefs、capabilityState、externalBlockers、previousEvidenceRef；runner 另记录 OS build/GPU/driver/Python/CUDA；未在授权硬件执行时 NOT_RUN/UNVERIFIED。

#### 10. 回滚

- 关闭并物理移除 Worker Pack；清理临时 AppContainer profile、ACL、Job root、进程和缓存。
- 不迁移项目数据，不更改系统驱动；清理失败产生独立 blocker，不能标 done。

#### 11. 完成定义

- 统一门禁：npm run format:check、npm run lint、npm run typecheck、npm test、npm run build:core 均有结构化结果；文档、追踪、许可审计和用户可见错误处理完成。

- format/lint/typecheck/integration/release:verify 有结果；矩阵、恶意测试、资源预算、清理与许可均可追踪。
- 用户错误明确区分 GPU 不可用与沙箱不安全，并说明 Core 手工链可继续。

#### 12. 退出状态

- 文档与探针 done 不等于指定硬件 VERIFIED；无授权 8GB 测试机时为 UNVERIFIED。
- AppContainer 安全为发布硬门；任一失败阻断 Worker Pack，但不阻断不含 Worker 的 Core。

## F2S-DEV-M00-005 — PSD与Spine 4.2.43 Spike

#### 1. 任务头

- 目标：用最小合法 fixture 验证最小分层 PSD 写出可行性、Spine 4.2.43 JSON 静态契约，以及用户合法 CLI/Editor 往返证据协议。
- 非目标：不捆绑 Spine 组件，不内置 atlas/spine/skel writer，不在无合法 Editor 时宣称兼容 VERIFIED。
- 预计：3d。

#### 2. 追踪

- FR/NFR：F2S-NFR-SPINE-001、F2S-FR-EXP-002、F2S-FR-EXP-003。
- ADR：F2S-ADR-SPN-001、F2S-ADR-PSD-001、F2S-ADR-CLI-001、F2S-ADR-SPN-003。
- 风险：PSD 库无法表达层/透明度、Spine patch 字段漂移、专有输出越权。
- 上游 DEV：F2S-DEV-M00-001、F2S-DEV-M00-002、F2S-DEV-M00-003；下游 DEV：F2S-DEV-M08-003、F2S-DEV-M08-006、F2S-DEV-M08-007。
- 携带项：P0/P1 无；P2 无。

#### 3. 输入

- fixture：M00-003 synthetic-character；独立最小两层 RGBA attachment。
- config：Spine Editor/CLI 4.2.43 路径仅由用户配置，缺失为 EXTERNAL。
- schema：F2S-PSD-SPIKE-001 v1、F2S-SPINE42-COMPAT-EVIDENCE-001 v1；hash 在运行后产生。

#### 4. 输出

- tools/spikes/m00/psd-minimal-probe.mjs：最小 PSD 探针，owner F2S-DEV-M00-005。
- fixtures/m00/spine42-probe/rig-ir.json、skeleton.json、attachments/body.png、attachments/weapon.png：开放格式最小 fixture，owner 本任务。
- docs/spikes/F2S-PSD-SPINE42-FEASIBILITY-001.md：兼容与许可裁决，owner 本任务。
- tools/spikes/m00/spine42-evidence-check.mjs：只校验用户工具证据，不写专有格式，owner 本任务。
- 再生性：工具与开放 fixture 是受版本控制源；可行性报告可从同号 evidence 重生，专有输出不进入 workspace。

#### 5. 设计边界

- 归属：PSD/Spine 均为可替换 Adapter Spike；Rig IR 是唯一内核边界。
- 使用：Adapter、Golden Master、Lease、Capability Evidence。
- 拒绝：UI 直调 CLI、使用 latest、内置官方 runtime、把 CLI 成功等同人工审批、伪造 atlas/spine/skel。

#### 6. work units

##### F2S-WU-M00-005-01 — 最小分层 PSD bytes 探针

- output: tools/spikes/m00/psd-minimal-probe.mjs
- reads: F2S-DEV-M00-003、fixtures/m00/synthetic-character/master.png、F2S-ADR-PSD-001、F2S-DEV-M00-002、docs/compliance/F2S-SUPPLY-INVENTORY-001.json
- writes: tools/spikes/m00/psd-minimal-probe.mjs
- steps: 1) 固定两层 RGBA 输入；2) 写最小 PSD；3) 用独立 parser 回读层名/尺寸/透明度；4) 比较像素 hash；5) 记录库版本许可；6) 删除不可接受候选。
- command: npm run test:integration
- tests: F2S-TST-GOLD-001；向量为两层正常、空层名、超尺寸、alpha 边缘、截断 bytes。
- evidence: evidence/M00/F2S-DEV-M00-005/F2S-WU-M00-005-01/psd-roundtrip.json 与 minimal.psd.sha256
- dependsOn: F2S-DEV-M00-002、F2S-DEV-M00-003
- parallelSafety: isolated
- rollback: 删除生成 PSD 和候选库登记；源 PNG 不变。
- estimate: 1d

##### F2S-WU-M00-005-02 — Spine 4.2.43 开放 fixture 与静态契约

- output: fixtures/m00/spine42-probe/
- reads: F2S-ADR-SPN-001、F2S-ADR-TIME-001、F2S-DEV-M00-003、fixtures/m00/synthetic-character/manifest.json
- writes: fixtures/m00/spine42-probe/rig-ir.json；fixtures/m00/spine42-probe/skeleton.json；fixtures/m00/spine42-probe/attachments/body.png；fixtures/m00/spine42-probe/attachments/weapon.png
- steps: 1) 建最小 bone/slot/skin；2) 固定 Spine skeleton version 4.2.43；3) 使用微秒整数到秒转换规则；4) 只引用透明 PNG；5) 规范排序与 float；6) 写 golden hash。
- command: npm run test:spine
- tests: F2S-TST-GOLD-001、F2S-TST-CONTRACT-005；4.2.43 通过，patch 错误、路径穿越、NaN/Infinity、缺 attachment 拒绝。
- evidence: evidence/M00/F2S-DEV-M00-005/F2S-WU-M00-005-02/static-contract.json 与 golden.sha256
- dependsOn: F2S-DEV-M00-001、F2S-DEV-M00-003
- parallelSafety: isolated
- rollback: 删除本目录 fixture 或恢复上一 golden；不修改用户项目。
- estimate: 1d

##### F2S-WU-M00-005-03 — 用户 Editor/CLI 往返证据裁决

- output: docs/spikes/F2S-PSD-SPINE42-FEASIBILITY-001.md
- reads: F2S-WU-M00-005-01、F2S-WU-M00-005-02、evidence/M00/F2S-DEV-M00-005/F2S-WU-M00-005-01/psd-roundtrip.json、evidence/M00/F2S-DEV-M00-005/F2S-WU-M00-005-02/static-contract.json、F2S-OQ-SPN-001、F2S-OQ-LGL-001
- writes: tools/spikes/m00/spine42-evidence-check.mjs；docs/spikes/F2S-PSD-SPINE42-FEASIBILITY-001.md
- steps: 1) 定义显式 CLI lease；2) 校验用户路径/版本/许可确认；3) 记录命令参数与输入输出 hash；4) 区分开放候选和 CLI-owned 输出；5) 无工具写 NOT_RUN/EXTERNAL；6) 汇总 PSD 与 Spine 裁决。
- command: npm run test:spine
- tests: F2S-TST-E2E-SPINE-001、F2S-TST-CONTRACT-005；合法 4.2.43、无工具、错 patch、租约过期、输出 hash 缺失。
- evidence: evidence/M00/F2S-DEV-M00-005/F2S-WU-M00-005-03/editor-roundtrip.json 与 feasibility-report.json
- dependsOn: F2S-WU-M00-005-01、F2S-WU-M00-005-02
- parallelSafety: shared-lock:user-spine-cli
- rollback: 撤销租约、删除临时导出；不删除用户工具，不把专有输出纳入仓库。
- estimate: 1d

#### 7. 正向验收

- F2S-TST-GOLD-001：PSD 回读层结构、尺寸、alpha 与像素 hash 一致；Spine JSON 静态 golden 稳定。
- F2S-TST-E2E-SPINE-001：用户合法 4.2.43 Editor/CLI 打开候选并产生完整往返证据，能力才可 VERIFIED。
- F2S-TST-CONTRACT-005：CLI lease、版本、输入输出 hash 和 writer provenance 完整，CLI-owned 文件数与声明一致。

#### 8. 负向与故障验收

- 无合法工具、错 patch、许可确认缺失、CLI 超时/取消/非零退出时为 NOT_RUN/EXTERNAL 或 FAILED，不能假 PASS。
- 发现内置 atlas/spine/skel writer、官方组件进入包、PSD 像素/层丢失、路径穿越或非有限数立即失败。

#### 9. 证据

- F2S-EVD-M00-005：evidence/M00/F2S-DEV-M00-005/evidence.json；同号任务证据索引。
- evidence.json 必填 taskId、status、command、exitCode、startedAtUtc、endedAtUtc、runner、toolVersions、inputHashes、outputHashes、logRefs、reportRefs、capabilityState、externalBlockers、previousEvidenceRef；toolVersions 精确写 4.2.43；无合法工具时 capabilityState=EXTERNAL。

#### 10. 回滚

- 移除未通过的 PSD/Spine Adapter 候选与临时输出，保留 Rig IR 和 PNG 源。
- 撤销 CLI lease，不修改用户安装/激活；项目数据尚未迁移。

#### 11. 完成定义

- 统一门禁：npm run format:check、npm run lint、npm run typecheck、npm test、npm run build:core 均有结构化结果；文档、追踪、许可审计和用户可见错误处理完成。

- format/lint/typecheck/integration/test:spine/release:verify 均有结构化结果；golden、许可、patch、provenance 和错误文案齐全。
- 任何外部 NOT_RUN 在 UI/报告中明确展示，不能被测试总通过掩盖。

#### 12. 退出状态

- PSD 能力仅在独立 parser 往返后 VERIFIED；Spine 能力仅在用户合法 4.2.43 实测后 VERIFIED。
- F2S-OQ-SPN-001、F2S-OQ-LGL-001 未关闭时阻断相关商业兼容声明，但不阻断开放 Rig IR/PNG/JSON 候选实现。
