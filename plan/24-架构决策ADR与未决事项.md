---
doc_id: F2S-DOC-ADR-001
revision: 2.8
status: reviewed
canonical_for:
  - F2S-ADR-REGISTRY-001
  - F2S-ADR-PROD-001
  - F2S-ADR-PROD-002
  - F2S-ADR-PLAT-001
  - F2S-ADR-STACK-001
  - F2S-ADR-ARCH-001
  - F2S-ADR-TIME-001
  - F2S-ADR-SPN-001
  - F2S-ADR-CLI-001
  - F2S-ADR-TARGET-001
  - F2S-ADR-SPN-003
  - F2S-ADR-INPUT-001
  - F2S-ADR-PSD-001
  - F2S-ADR-ACTION-001
  - F2S-ADR-AST-001
  - F2S-ADR-WEAPON-001
  - F2S-ADR-GPU-001
  - F2S-ADR-SANDBOX-001
  - F2S-ADR-STO-001
  - F2S-ADR-JOB-001
  - F2S-ADR-LIC-001
  - F2S-ADR-WAIVER-001
  - F2S-ADR-HITL-001
  - F2S-ADR-REL-001
  - F2S-ADR-PLT-001
  - F2S-ADR-INP-001
  - F2S-ADR-ACT-001
  - F2S-ADR-WPN-001
  - F2S-ADR-NET-001
  - F2S-ADR-REV-001
  - F2S-ADR-SPN-002
  - F2S-ADR-REJ-001
  - F2S-ADR-REJ-002
  - F2S-ADR-REJ-003
  - F2S-ADR-REJ-004
  - F2S-ADR-REJ-005
  - F2S-ADR-REJ-006
  - F2S-ADR-REJ-007
  - F2S-ADR-REJ-008
  - F2S-ASM
  - F2S-OQ
depends_on: [F2S-DOC-GOV-001, F2S-DOC-CHARTER-001, F2S-DOC-REQ-001, F2S-DOC-ARCH-001]
review_score_ref: F2S-SCORE-DOC-ADR-001-R3B
last_verified: 2026-07-11
---

# 架构决策 ADR 与未决事项

## 1. 决策治理

本文件记录跨文档、难以逆转或影响商业边界的决定。字段分为：`decision_state=proposed|accepted|superseded|rejected`与`evidence_state=not_run|source_checked|partial|verified|external_blocked`。涉及外部来源事实的ADR进一步拆成同一枚举的`source_evidence_state`和`implementation_evidence_state`，二者不得合并推导；实现门只读取后者。`accepted`只表示已决定采用，不等于实现验证；只有具有原子任务登记、可追溯且精确编号的`F2S-EVD-Mxx-yyy`真实证据，implementation evidence才能为`verified`。旧行的“状态：accepted”只读作decision_state，不能推导evidence_state。

`F2S-ADR-REGISTRY-001` 是ADR唯一owner registry。跨领域、产品、许可和不可逆决策的canonical body在本文件；实现局部ADR的canonical body可以留在指定领域文档，但其exact ID、owner和状态必须在下表登记。裸`F2S-ADR-*`前缀不构成owner，未登记的新ADR无效。

| Implementation-local ADR exact IDs | Canonical owner | Registry status |
| --- | --- | --- |
| F2S-ADR-ARCH-002、F2S-ADR-ARCH-003、F2S-ADR-ARCH-004、F2S-ADR-ARCH-005 | F2S-DOC-ARCH-001 | accepted/not_run |
| F2S-ADR-ENV-001、F2S-ADR-ENV-002、F2S-ADR-ENV-003、F2S-ADR-ENV-004、F2S-ADR-ENV-005 | F2S-DOC-ENV-001 | accepted/not_run |
| F2S-ADR-RENDER-001、F2S-ADR-RENDER-002、F2S-ADR-RENDER-003 | F2S-DOC-RENDER-001 | accepted/not_run |
| F2S-ADR-DOMAIN-001、F2S-ADR-DOMAIN-002、F2S-ADR-DOMAIN-003 | F2S-DOC-DOMAIN-001 | accepted/not_run |
| F2S-ADR-PIPE-001、F2S-ADR-PIPE-002、F2S-ADR-PIPE-003 | F2S-DOC-PIPE-001 | accepted/not_run |
| F2S-ADR-DATA-001、F2S-ADR-DATA-002、F2S-ADR-DATA-003 | F2S-DOC-STORE-001 | accepted/not_run |

变更 ADR 必须说明受影响的需求、接口、schema、测试、任务、迁移和发布产物。不得直接修改结论而删除旧取舍。

## 2. 已接受决策

### F2S-ADR-PROD-001 — 闭源商业产品

- 决策状态：accepted；证据状态：not_run；依据：用户决策 #1。
- 决策：产品以闭源商业软件发布；所有随产品分发的代码、模型、字体、图标、fixture和安装组件必须通过F2S-LIC-POLICY-001。
- 后果：研究用途、Non-Commercial、未知许可和不可追踪资产不得进入发布包。

### F2S-ADR-PROD-002 — 完整 Production Assist

- 决策状态：accepted；证据状态：not_run；依据：用户决策 #2。
- 决策：V1提供从图片接收、规格、提示词、分层、Rig、动作审核到导出的完整人机协作工作流，不定位为无人值守Quick Rig。
- 后果：人工审批、失败恢复和编辑能力属于P0；“任意单图全自动商业级重度动作”明确不在承诺内。

### F2S-ADR-PLAT-001 — Windows local-first 桌面产品

- 决策状态：accepted；证据状态：not_run；依据：用户决策 #1、#6、#7。
- 背景：产品为闭源商业 Production Assist，图片不得发送第三方云。
- 决策：Windows 11 x64 为一级目标，使用本地桌面进程和本地项目目录；Windows 10 22H2仅尽力兼容。MVP 不实现账号、云同步、多人实时协作或遥测。
- 备选：公共 Web/SaaS、Electron、纯浏览器。
- 取舍：本地文件和GPU集成更可控，但需要 Windows 安装、WebView2、GPU/驱动诊断。
- 验证：F2S-TST-E2E-008、F2S-EVD-M10-005干净机证据。

### F2S-ADR-STACK-001 — Tauri + React/TypeScript + PixiJS + Rust + 可选 Python Worker

- 决策状态：accepted；证据状态：not_run。
- 决策：Tauri 2 管理桌面边界；React/TypeScript负责UI；PixiJS 8负责画布；Rust负责领域/Application/存储/Job/安全；Python仅作为可替换推理Worker。
- 备选：Electron全TS、Python桌面、C++/Qt、纯Rust UI。
- 取舍：多语言增加构建复杂度，但隔离AI生态和商业核心；Python不是项目事实源。
- 限制：Worker可完全缺失，基础手工编辑、Rig和导出仍可使用。
- 验证：F2S-EVD-M00-004子进程/IPC Spike、F2S-EVD-M01-001与F2S-EVD-M02-005契约证据。

### F2S-ADR-ARCH-001 — 六边形架构与 Rig IR 唯一事实源

- 决策状态：accepted；证据状态：not_run。
- 决策：Domain/Application 不依赖 UI、文件、数据库、AI、Spine；所有外部格式通过端口/Adapter。Rig IR 与版本化项目manifest是权威数据，Spine JSON只是派生输出。
- 命令幂等边界：全项目索引键固定为`(actorId,idempotencyKey)`，`commandKind`不得成为隐藏命名空间。只有首次commandKind与canonical payload都一致时才返回首次不可变结果；同key异payload或跨commandKind复用一律是payload conflict，并在任何CAS前fail closed，产生零record、零registry/stream transition、零project revision迁移。诊断必须保留首次/本次commandKind和payload hash以便定位，UI不得静默换key重试；用户确认的新意图必须生成新key。
- 备选：以Spine JSON为内部模型、前端store为事实源。
- 后果：需维护schema和映射，但可替换导出器、Worker和渲染器，避免第三方格式蔓延。
- 验证：schema往返、依赖边界检查、golden export。

### F2S-ADR-TIME-001 — 整数 tick 与有理时间基

- 决策状态：accepted；证据状态：not_run。
- 决策：RuntimeSpec、MotionSpec、Rig IR、IPC和深链持久化一律使用有符号i64 tick；JSON/NDJSON中的 tick 必须是满足 `0|-?[1-9][0-9]*` 的十进制字符串，禁止 JSON number、前导加号、前导零和 `-0`。动画时间进一步限制为非负。
- `timeBase={numerator:u32,denominator:u32}` 两项使用 JSON integer，必须大于零且 `gcd(numerator,denominator)=1`；默认唯一规范形为 `1/30000 s`。载入非约分形式时拒绝并给出迁移建议，不能静默改写从而改变 CAS hash。
- 秒和显示帧仅为派生值。十进制秒导入只接受无指数、最多64字节的规范十进制文本，以精确有理数和 checked `i128` 中间值计算并使用 round-half-to-even 量化；provenance记录原文本、量化 tick 与精确舍入差。任何结果超出 i64 均拒绝。
- Canonical ULID标量：本项目所有schema类型`Ulid`的wire固定grammar为`^[0-7][0-9A-HJKMNP-TV-Z]{25}$`，恰好26位大写Crockford Base32；首字符`0–7`保证128-bit不溢出，明确排除`I/L/O/U`。小写、连字符、空白、错误长度、首字符`8–Z`、`O→0`或`I/L→1`别名、任何宽松解码/规范化均拒绝；decode后重新encode必须与原wire逐字相等。生成器在同一进程/毫秒内必须使用monotonic ULID，最终wire仍须先通过同一validator再参与JCS/hash。任何错误在读取或命令边界fail closed，错误证据保存字段/JSON Pointer、字符位置和原因，不迁移、不部分提交。
- 只有4.2.43 Export Adapter可在边界转换秒：V1单动画的精确有理时长必须在0至60秒内；`tick*numerator`先提升为checked i128，再除以denominator。序列化结果最多9位小数、half-even、无指数、无尾零且规范化负零，并对同一timeline逐项进行文本值和IEEE-754 binary32解析值的有限性、顺序与碰撞检查；不同tick若在任一层相等或逆序则导出失败。转换值不得写回Rig IR。
- 备选：全链`f64`秒、以显示帧号作为事实源。
- 后果：需要统一舍入、大整数、格式化和解析后碰撞测试，但避免显示FPS变化、JS安全整数与跨语言浮点漂移。官方4.2 spine-libgdx读取JSON数值使用`getFloat`，因此binary32预检是无Editor时的保守代理，不替代本地4.2.43 CLI往返证明：<https://github.com/EsotericSoftware/spine-runtimes/blob/4.2/spine-libgdx/spine-libgdx/src/com/esotericsoftware/spine/SkeletonJson.java>。
- 验证：F2S-TST-PROP-002、F2S-TST-CONTRACT-004、F2S-TST-CONTRACT-005、F2S-TST-GOLD-004；F2S-EVD-M07-001、F2S-EVD-M08-004、F2S-EVD-M08-007。

### F2S-ADR-SPN-001 — 精确固定 Spine Editor 4.2.43

- 决策状态：accepted；source_evidence_state：source_checked；implementation_evidence_state：not_run。
- 来源：用户决策 #9、官方 Changelog 2026-07-11核验。
- 决策：数据目标为4.2，Editor/CLI固定4.2.43；禁止生产使用 `latest`、`lateststable`、`4.2.xx`或beta。JSON写入 `spine: 4.2.43`。
- 后果：升级必须另立ADR、全量golden与迁移；补丁不随安装机漂移。
- 来源：<https://esotericsoftware.com/spine-changelog>；实现证据目标为F2S-EVD-M08-007，只有该证据索引中的真实4.2.43往返文件存在且通过后才可改为verified。

### F2S-ADR-CLI-001 — 每次验证进程钉扎4.2.43并产出实际patch证明

- 决策状态：accepted；source_evidence_state：source_checked；implementation_evidence_state：not_run。
- 决策：probe只做离线`--version`；真正的import/export验证进程必须在应用互斥lease下显式选择`--update 4.2.43`，转换阶段无网络grant，并从本轮重导产物/输出再次证明实际patch。probe结果不能跨进程充当证明。
- 理由：用户手工Editor会话和active patch会造成TOCTOU；退出码也不能证明实际数据版本。
- 后果：本机未缓存合法4.2.43时验证失败并保持UNVERIFIED；获取/切换patch需要独立用户联网许可。
- 验证：F2S-TST-CONTRACT-005、F2S-TST-E2E-SPINE-001、F2S-EVD-M08-006、F2S-EVD-M08-007。

### F2S-ADR-TARGET-001 — 仅 Spine Editor 为首个集成目标

- 决策状态：accepted；证据状态：not_run；Decision Owner：Architecture/Product。
- 可审计决策链（Asia/Shanghai，2026-07-11）：①用户第10项原文要求“必须在 Unity、Godot、Cocos、自研或仅 Spine Editor 中选一个”；②架构负责人以最小许可面、最小集成面和既定交付物为理由，明示选择“仅 Spine Editor 4.2.43”，同时提供更改目标的opt-out；③用户在该披露后回复“继续”。本记录表示用户授权下的项目方选择，不表述为用户亲自点名。
- 决策：V1不实现Unity/Godot/Cocos/自研Runtime集成。通用内置导出包含Rig IR/最小PSD；内置Spine边界只输出Spine JSON、透明attachment PNG和atlas packing/input manifest，并可调用用户本地Professional CLI 4.2.43生成/验证其专有格式。
- 备选：选择一个游戏引擎；捆绑 Spine Runtime 因许可边界不是当前可接受方案。
- 取舍：避免引擎和专有Runtime许可扩张；实机游戏验证推迟。
- 退出：若用户改选其他目标，必须以新ADR supersede 本决策，并补目标 SDK、Runtime许可、运行时验收和路线图，不可在当前V1静默扩张。

### F2S-ADR-SPN-003 — 不捆绑 Spine Editor/Runtime，专有三格式仅由本地CLI生成

- 决策状态：accepted；证据状态：not_run。
- 决策：内部预览渲染自有Rig IR；发布包不含官方Runtime、Editor或激活信息。内置Spine Adapter只能生成4.2.43 JSON、透明attachment PNG和`atlas-input-manifest.json`，该manifest只描述packing输入/策略。`.atlas/.spine/.skel`没有内置writer，仅可由用户本地合法Professional CLI 4.2.43在受控operation中生成；自动路径为Rig IR→内置开放输入→CLI，禁止fallback writer。
- 备选：嵌入spine-ts；逆向项目文件；clean-room runtime。
- 理由：用户许可白名单和开发工具商业边界；`.spine`没有第三方writer规范，`.atlas/.skel`也不得由未验证自研实现冒充本地Professional产物。
- 后果：无CLI时仍可交付Rig IR/PSD/JSON/透明PNG/atlas-input manifest，但只能标`exported_unverified`且专有三格式数量为0。任何专有格式必须逐文件绑定本轮CLI evidence、实际patch与hash；writer ownership进入golden fixture硬断言。

### F2S-ADR-INPUT-001 — 只接收图片，不内置生图API

- 决策状态：accepted；证据状态：not_run；依据：用户决策 #5、#6、#13。
- 决策：输入为角色母版、可选动作关键帧图片、动作描述文本；工具不调用第三方Image API，不上传公共云。
- 后果：工具提供可复制的动作提示词包，用户在外部生成后再导入；无法保证外部生成器身份一致性，必须做导入对比和审批。

### F2S-ADR-PSD-001 — PSD仅输出且必须满足最小可重建契约

- 决策状态：accepted；证据状态：not_run。
- 决策：V1输入只支持PNG/JPEG/WebP，PSD/PSB一律无副作用拒绝。输出PSD必须由PsdExportProfile/PsdManifest逐项证明画布、组/层级、名称、顺序、visibility、opacity/alpha、origin/pivot与像素层，并由独立parser reopen；高级样式可报告降级，P0字段不可降级。
- 备选：直接接收任意PSD；只写扁平PSD占位。
- 后果：Rust是唯一文件提交者；任何TS writer只返回受限byte stream。writer/parser许可、内存、Photoshop/Spine UI导入证据通过前只可Candidate/UNVERIFIED。
- 验证：F2S-TST-E2E-007、F2S-EVD-M08-003。

### F2S-ADR-ACTION-001 — 固定 V1 动作集和三阶段动作语义

- 决策状态：accepted；证据状态：not_run；依据：用户决策 #3。
- 决策：实体ID严格为`F2S-ACT-IDLE-001`、`F2S-ACT-RUN-001`、`F2S-ACT-JUMP-001`、`F2S-ACT-FALL-001`、`F2S-ACT-DASH-001`、`F2S-ACT-ATTACK-001`、`F2S-ACT-ATTACK-002`、`F2S-ACT-ATTACK-003`、`F2S-ACT-HIT-001`、`F2S-ACT-DEATH-001`；对应clip name为`idle/run/jump/fall/dash/attack_01/attack_02/attack_03/hit/death`，总数恰好10。攻击按 anticipation/active/recovery，命中与取消等玩法元数据必须人工批准。
- 说明：用户表述为上述10类，其中attack×3展开后总计10个clip；任何额外转场或变体需变更范围。
- 后果：AssetSpec和提示词必须覆盖每个clip；不推断未批准的游戏真值。

### F2S-ADR-AST-001 — 母版驱动、按表征缺口补料

- 决策状态：accepted；证据状态：not_run。
- 决策：批准母版是视觉真值；可见像素优先原样复制，只补隐藏区。先生成最小分层和临时Rig，压力测试后才增加corrective/pose/sequence素材。
- 备选：每个动作独立生成整人图；一次性生成所有附件。
- 后果：需要candidate/approved revision和缺料闭环，但显著降低身份漂移与组合爆炸。

### F2S-ADR-WEAPON-001 — 使用 `primary_weapon` 领域占位，不假定具体武器

- 决策状态：accepted；证据状态：not_run；项目参数`F2S-OQ-WPN-001`仍开放。
- 背景：用户确认单武器但未给类型。
- 决策：schema使用WeaponSpec和`primary_weapon`；内置提示词不得写死剑。示例fixture可用无品牌单手训练剑，但不影响产品默认。
- 退出：用户给出具体武器后新增项目级WeaponSpec，不需要改schema。

### F2S-ADR-GPU-001 — 本地计算和私有远程 GPU

- 决策状态：accepted；证据状态：not_run；依据：用户决策 #6、#8的协调解释。
- 决策：默认网络关闭；禁止第三方SaaS和遥测。可配置本机、局域网或企业自托管HTTPS端点；每项目显式开启，显示目标和上传清单，支持TLS/证书指纹/短期凭据。
- 后果：前端无网络权限，Rust网络Adapter统一执行策略；公网地址默认拒绝，例外需企业策略。

### F2S-ADR-SANDBOX-001 — 商业D2 Worker唯一使用windows-appcontainer-v1

- 决策状态：accepted；证据状态：not_run。
- 决策：处理D2项目数据的商业本地Worker必须同时具备restricted AppContainer token、无network capability、专用ACL Job root、Job Object子进程/资源约束与OS egress/路径/句柄/breakaway恶意探针；Profile固定为`windows-appcontainer-v1`，不存在“ACL或token”的等价降级。
- 备选：policy_only、仅防火墙、普通用户Python、任选隔离组合。
- 后果：任一控制不可用就禁用Provider且物理移除Worker Pack；手工核心保持可用。
- 验证：F2S-TST-RGPU-005、F2S-TST-E2E-SEC-005、F2S-EVD-M09-001。

### F2S-ADR-STO-001 — 可搬移项目目录为真相、索引可重建

- 决策状态：accepted；证据状态：not_run。
- 决策：版本化JSON+不可变artifact为源真相；redb只做任务/搜索/缩略图索引，可删除重建；内容寻址、原子替换、单写者锁和恢复journal。
- 备选：SQLite或单ZIP为唯一存储。
- 取舍：文件可审计和恢复，项目目录文件数较多；打包ZIP只作导入导出。

### F2S-ADR-JOB-001 — Job succeeded与领域candidate分离

- 决策状态：accepted；证据状态：not_run。
- 决策：Job`succeeded`只表示输出完成schema/hash/安全/完整provenance校验并以未绑定`JobOutputArtifact`进入CAS；另一个带expected project revision的`Register*Candidate`事务才能创建领域candidate。失败注册不改写Job终态，输出在保留期内可重试绑定。
- 备选：Worker直接写candidate；把CAS提升、项目revision和Job终态混成一个跨进程事务。
- 后果：取消/success由Rust单一仲裁，崩溃可从JobExecutionRecord恢复；UI必须展示“注册为候选”，不能把succeeded显示为已应用/批准。
- 验证：F2S-TST-095、F2S-TST-IPC-004、F2S-EVD-M02-004。

### F2S-ADR-LIC-001 — 发布依赖采用可审计宽松许可白名单

- 决策状态：accepted；证据状态：not_run；依据：用户决策 #12。
- 决策：具体allowlist和分类只由F2S-LIC-POLICY-001定义；本ADR批准“经审计宽松许可+显式外部系统分类”的政策，不在此复制可能漂移的列表。GPL/LGPL/AGPL/MPL/SSPL/NC/Research-only、未知/无许可证禁止进入发布物。
- 分类：开源发布依赖、模型权重、系统/硬件运行时、用户商业工具、构建期工具分别审计；Windows/WebView2/CUDA/Spine属于系统或用户外部条件，不伪装成开源依赖。
- 后果：所有权重、字体、图标和训练数据声明单独登记；未知项fail closed。

### F2S-ADR-WAIVER-001 — 商业发布只允许P2例外

- 决策状态：accepted；证据状态：not_run。
- 决策：severity与priority正交；RuleDescriptor决定required subjects，02 baseline或不可变DefectRecord origin/floor经PriorityRegistry认证，stable DependencySnapshot只绑定实际PrioritySlice，full root是transport observation。P0/P1及全部硬类别不可waive。只有认证全P2非硬warning可走append-only grant/revoke；每次先分配全项目唯一exact waiverId/proposalId，proposal固定RuleCatalog/RuleSet、finding、target/dependency、slice、policy、scope、修复/回滚、priority/actor observations、clock epoch/checkpoint与期限。product/quality/release三个不同active actor及三个不同credential必须各自签署同一规范化签名消息；该消息的domain separation同时绑定exact waiverId、exact proposalId与proposalHash。任一ID/hash不一致、错proposal、跨waiver复用、同credential伪三人或grant事后改字段均零grant。
- 固定签名策略：本ADR采纳18号冻结的`ActorAttestationPolicyV1` authoritative JCS bytes：`{"allowedSignatureAlgorithms":["ecdsa-p256-sha256","rsa-pss-sha256"],"hashAlgorithmId":"f2s-actor-attestation-policy-sha256-jcs-v1","organizationEnrollmentRequired":true,"policyId":"f2s-actor-attestation-policy-v1","policyVersion":"1.0.0","requireCurrentActorRoleEvidence":true,"requireDistinctActors":true,"requireDistinctCredentials":true,"requireSameProposal":true,"roles":["product","quality","release"],"schemaVersion":"1.0.0","trustBundleRollbackMode":"failClosed"}`。其普通SHA-256/CAS artifact hash固定为`561d25223c507fe89f680ac5a1327c2c9500a85524e6929c90ae8d79eeef9cad`，domain hash固定为`b370ab9a28a68c010297e706e71d62c063bc97167494a051439fc1b3328c0d90`；启动、proposal、grant、binding、active replay和release signer都必须与签名application manifest一起自校验，任一bytes/hash不符即禁用waiver。字段变化只能发布新policy版本，不能沿用这些hash。
- 固定算法注册表：`SignatureAlgorithmRegistryV1`采用18号冻结的单行UTF-8/JCS bytes（末尾无换行）：`{"algorithms":[{"curve":"P-256","encoding":"ieee-p1363","hash":"sha-256","id":"ecdsa-p256-sha256","lowSRequired":true,"signatureLengthBytes":"64"},{"hash":"sha-256","id":"rsa-pss-sha256","mgf":"mgf1-sha256","minimumModulusBits":"2048","saltLengthBytes":"32","trailerField":"1"}],"base64urlPadding":"forbidden","hashAlgorithmId":"f2s-signature-algorithm-registry-sha256-jcs-v1","registryId":"f2s-signature-algorithm-registry-v1","schemaVersion":"1.0.0","strictCanonicalDecoding":true}`。普通SHA-256/CAS artifact hash固定为`43185e9a1a5bdf5a5ee4b6e3ce13a7c27852d1c8487ea9ec4125094cd54b5ac8`，`SHA256("f2s-signature-algorithm-registry-v1\0" || registry bytes)`固定为`1419106463bf0193c2885c90ef68520914ea426d3c3ffe1848fce6b7acd77c40`；application manifest、producer和所有validator必须钉住并自校验两者。
- 算法精确语义：ECDSA只接受64-byte IEEE-P1363 `r||s`、P-256/SHA-256和low-S，DER、错长度、r/s为0或越界、high-S一律拒绝；RSA-PSS只接受SHA-256、MGF1-SHA256、saltLen=32、trailerField=1且modulus≥2048 bits。`signatureBase64url`只接受RFC 4648 URL-safe alphabet、禁止`=`padding，拒绝空白、非最短表示及解码后长度错误；producer/validator不得先宽松接受再规范化。
- 可信bootstrap：签名application manifest只钉住publisher enrollment验证key；`OrganizationTrustEnrollmentV1`必须由该key签名并per-organization/per-product-edition绑定organizationId、productId/edition、组织offline-root keyId/SPKI/算法和有效期。ActorRegistry唯一sequence=0 empty genesis要求enrollment/bundle/change-authorization双hash全null、bundle sequence=0且actors为空；首次启用只能由单一sequence=1 `enrollOrganization` transition原子固定enrollment双hash、两条sequence=1 bundle roots、bootstrap authorization和initial actors，部分失败仍为empty genesis。零配置、无/伪造/过期/尚未生效/错edition或根不匹配enrollment均禁用governed waiver。
- 项目不可换绑：同一`ProjectId`在首次enroll后必须永久固定enrollment双hash，后继transition不得替换、续期成另一pack或迁移组织/edition。到期或迁移只能克隆内容到新`ProjectId`，重新enroll、grant和逐target审批；旧项目/证据只读保留，旧ActorRegistry、grant、binding、approval及其active结论不得迁移。
- 身份信任：ActorRegistry只能锚定当前`ActorTrustBundle`与`ActorRevocationBundle`的sequence/artifact/domain三元组；两条bundle均为离线root签名、sequence+1、previous artifact/domain双链。trust anchor禁用/撤销和credential/SPKI/serial撤销只能单调增加，禁止rollback、删撤销、复活anchor或用旧bundle验证active binding。相关credential撤权使旧binding永久inactive，无关entry推进不误失效。
- 离线anti-rollback边界：V1只在当前项目根、manifest/registry链、clock sidecar、应用状态和已导入bundle未被特权攻击者整体替换时，承诺检测项目内分叉、删撤销、旧head替换和局部复制。它无法知道组织已签发但操作者从未导入的更高sequence bundle，也不对抗本机管理员或磁盘快照把项目、sidecar、应用状态和导入bundle全套回滚。需要global freshness或特权回滚对抗时必须新建V2 ADR并引入在线transparency、组织HSM单调计数器、TPM或远端锚；不得扩大V1声明。
- 时间：ClockRecovery只允许`oldEpoch→oldEpoch+1`并创建全项目checkpoint sequence+1、prev双指旧epoch head的epoch-boundary successor，old/new checkpoint双head与唯一RecoveryRecord交叉固定；prev-null、跳sequence或伪epoch拒绝。
- 固定发布覆盖：`ReleaseGateCoverageReportV1`固定`schemaVersion="1.0.0"`、`hashAlgorithmId="f2s-release-gate-coverage-report-sha256-jcs-v1"`、projectRevision、requiredTargetSetHash、排序唯一`targets[{gate,targetStableIdentity,currentTargetRef,evaluationInputsHash,outcomeHash,approvalRecordArtifactHash,approvalRecordHash,ackRefs[],bindingRefs[],currentValidationEvidenceRefs[],decision}]`和reportHash。required target set只能从current项目配置枚举全部Master/Specs/AssetPlan/Layers/Rig、十动作Pose+Gameplay及全部交付ExportProfile，不能由请求payload删减；`requiredTargetSetHash=SHA256("f2s-release-required-gate-target-set-v1\0" || JCS(sorted [{gate,targetStableIdentity}]))`，targets必须与该排序集合恰好全等。每项必须从current ApprovalStreamRegistry head重放出active approve，并以artifact/domain双hash引用current精确ack或waiver validation；P0/P1/blocker必须为0、每个governed P2 warning恰有一个active binding、每个艺术advisory warning恰有一个active ack。`reportHash=SHA256("f2s-release-gate-coverage-report-v1\0" || JCS(report without reportHash))`，完整bytes另存普通artifact hash，release manifest双引用两者；缺项、旧head、stale/inactive、未覆盖warning、policy/enrollment失效或任一hash错均阻断签名，测试通过不能替代人工覆盖。
- Stable freshness声明与canonical 15分钟边界：若上述coverage任一required target依赖governed waiver，签名前必须由用户显式执行“刷新组织bundle”，导入组织提供的文件，并由current ActorRegistry中的release角色credential签署`OrganizationBundleFreshnessDeclarationV1`。每次Stable签名作业生成新的ULID `signingAttemptId`；声明与最终release manifest必须逐字绑定同一attempt，声明另逐字段绑定ProjectId、release candidate、enrollment、ActorRegistry、trust/revocation bundle sequence/artifact/domain/fingerprint/issuedAt、ClockEvidence/effectiveNow及`limitationAcknowledged=true`，且`declaredLatestAt=ClockEvidence.effectiveNow`。signer在最终manifest签名前必须重读current effectiveNow并强制`0ns <= currentEffectiveNow-declaredLatestAt <= 900s`；900秒端点有效，负差或大于900秒无效。attempt变化、签名作业重启/重试、超时、旧candidate复用、错root/clock/signature或未确认限制都必须重新刷新并重签，旧attempt声明零复用。该声明只证明操作者声明已按组织流程刷新并看到这些head，不证明global latest。无governed waiver时不要求声明，release manifest中对应artifact/domain双hash必须为null。
- 备选：允许“非核心P1”书面接受；按Warning严重度直接接受。
- 后果：P1不交付须先范围变更并保留retired tombstone，不能降级。FindingDraft须登记固定default/history floor后重评。ApprovalRecord保存static binding；active每次按current priority slice subjects、固定ActorAttestationPolicy与SignatureAlgorithmRegistry、当前enrollment、waiver/proposal/三签名credential、trust/revocation bundles、actor roles及ClockCheckpoint验证，历史checkedAt不作真值。enrollment无效、过期/撤销/错scope/cross-boot旧epoch使waiver-dependent批准inactive；ClockRecovery后必须新grant/重审，无waiver批准不受影响。release manifest必须双引用current coverage/policy/algorithm-registry/enrollment/registry/bundle/freshness-declaration/clock/binding/validation evidence；无关entry不误失效，许可证/SLA不产生binding。
- 验证：F2S-TST-096、F2S-TST-099、F2S-TST-E2E-LIC-002、F2S-EVD-M11-004；freshness负测必须覆盖同attempt的0ns/900s端点通过、900s+1ns与负差拒绝，以及旧attempt、换attempt、作业重启、另一candidate声明全部拒签。

### F2S-ADR-HITL-001 — 人工审批是不可绕过的领域门

- 决策状态：accepted；证据状态：not_run；依据：用户决策 #11。
- 决策：母版、分层、骨骼、关键姿势、命中帧和最终QA必须审批；GateEvaluator通过绑定gate/target的decision-free纯Tracked resolver冻结闭包。审批绑定target、DependencySnapshot/PrioritySlice、stable outcome、policy、ack和static binding；每target独立stream由ApprovalStreamRegistry扎根，transport revision/full registry observation只用于CAS/审计。
- 后果：AI只能排序/报警。ack/static-waiver/approve独立于base评估；无关revision/registry entry不自失效，关联依赖/slice/policy/finding才失效。ClockRecovery通过连续epoch-boundary successor使旧epoch grants及waiver-dependent approvals永久inactive，无waiver批准不受影响。ReleaseGateCoverage枚举全部required active approval，不能用测试通过、单一“最终QA已批”或请求payload删减替代逐target人工门。

### F2S-ADR-REL-001 — 双击入口启动正式产品而非隐式安装

- 决策状态：accepted；证据状态：not_run；依据：用户决策 #16。
- 决策：发布产物包含Windows正式exe/安装器；仓库根目录提供可双击入口，定位已构建应用并启动，缺失时显示诊断。入口不联网下载、不执行任意脚本、不打开可见终端窗口。
- 验证：中文、空格、长路径和干净机测试。

### 2.1 ADR影响、验证与回滚追踪

下表补齐每个重大ADR对需求、接口/schema、测试、任务/证据和回滚的影响。章节正文与本表任何一处改变都要同步另一处；validator拒绝缺列ADR。

| ADR ID | Requirements | Interface / schema | Verification | DEV / EVD | Rollback / supersede trigger |
| --- | --- | --- | --- | --- | --- |
| F2S-ADR-PROD-001 | F2S-NFR-LIC-001、F2S-NFR-LIC-002 | F2S-LIC-POLICY-001 / release manifests | F2S-TST-E2E-LIC-001、F2S-TST-E2E-LIC-002 | F2S-DEV-M09-004 / F2S-EVD-M09-004 | 改变商业模式须新产品ADR与全供应链复审 |
| F2S-ADR-PROD-002 | F2S-FR-LAYER-001、F2S-FR-RIG-001、F2S-FR-ANIM-001 | Workflow/Gate/Approval schemas | F2S-TST-E2E-001、F2S-TST-E2E-005 | F2S-DEV-M11-001 / F2S-EVD-M11-001 | 缩减范围先改F2S-SCOPE-001且不得保留完整Production Assist声明 |
| F2S-ADR-PLAT-001 | F2S-NFR-COMPAT-001、F2S-NFR-COMPAT-002 | F2S-ENV-WIN-001/002、installer | F2S-TST-E2E-COMPAT-001、F2S-TST-E2E-COMPAT-002 | F2S-DEV-M10-005 / F2S-EVD-M10-005 | 新增平台须新ADR；现有Windows包回滚上一签名版本 |
| F2S-ADR-STACK-001 | F2S-NFR-MAINT-001、F2S-FR-APP-001 | F2S-IFC-001、F2S-IFC-002、F2S-IFC-003、F2S-IFC-004、F2S-IFC-005、Worker wire | F2S-TST-060、F2S-TST-070 | F2S-DEV-M01-001 / F2S-EVD-M01-001 | 替换Adapter可回滚；替换主栈须迁移ADR |
| F2S-ADR-ARCH-001 | F2S-NFR-MAINT-001、F2S-FR-EXP-001 | Rig IR/project schemas、ports、CommandEnvelope与全项目幂等索引 | F2S-TST-060、F2S-TST-PROP-002 | F2S-DEV-M02-001 / F2S-EVD-M02-001 | 保留旧reader/adapter，禁止绕过IR；跨commandKind同key冲突必须零迁移 |
| F2S-ADR-TIME-001 | F2S-FR-ANIM-001、F2S-FR-ANIM-002、F2S-FR-EXP-003 | RuntimeSpec/MotionSpec/Rig IR/IPC/Spine42Adapter、CanonicalUlid wire scalar | F2S-TST-PROP-002、F2S-TST-CONTRACT-004、F2S-TST-CONTRACT-005、F2S-TST-GOLD-004 | F2S-DEV-M07-001、F2S-DEV-M08-004、F2S-DEV-M08-007 / F2S-EVD-M07-001、F2S-EVD-M08-004、F2S-EVD-M08-007 | time或ULID grammar改变需新schema、链式迁移与全golden；现行非法wire零迁移 |
| F2S-ADR-SPN-001 | F2S-NFR-SPINE-001、F2S-FR-EXP-003 | Spine42 JSON/transparent-PNG/atlas-input exporter profile | F2S-TST-E2E-SPINE-001、F2S-TST-GOLD-001 | F2S-DEV-M08-007 / F2S-EVD-M08-007 | 新patch=新Adapter+迁移；失败保持4.2.43/UNVERIFIED |
| F2S-ADR-CLI-001 | F2S-FR-EXP-005、F2S-FR-EXP-006、F2S-FR-EXP-007 | SpineCliLease/SpineCliEvidence/CLI-owned `.atlas/.spine/.skel` | F2S-TST-CONTRACT-005、F2S-TST-075 | F2S-DEV-M08-006 / F2S-EVD-M08-006 | 禁用CLI仍可输出开放candidate；专有三格式必须为0 |
| F2S-ADR-TARGET-001 | F2S-SCOPE-001、F2S-NFR-SPINE-001 | Export target registry | F2S-TST-E2E-007 | F2S-DEV-M00-001 / F2S-EVD-M00-001 | 改选目标须新ADR、许可审查和目标运行时验收 |
| F2S-ADR-SPN-003 | F2S-FR-PREV-001、F2S-FR-EXP-003、F2S-NFR-LIC-002 | atlas-input manifest/package inventory/CLI writer provenance | F2S-TST-065、F2S-TST-E2E-SPINE-001 | F2S-DEV-M09-004、F2S-DEV-M08-007 / 同号EVD | 发现官方组件或内置专有格式writer即移除并阻断发布 |
| F2S-ADR-INPUT-001 | F2S-FR-IMP-001、F2S-FR-PROMPT-002 | SourceArtifact/import schemas | F2S-TST-100、F2S-TST-109 | F2S-DEV-M03-002 / F2S-EVD-M03-002 | 新增输入/API须scope+privacy ADR |
| F2S-ADR-PSD-001 | F2S-FR-EXP-002 | PsdExportProfile/PsdManifest | F2S-TST-E2E-007 | F2S-DEV-M08-003 / F2S-EVD-M08-003 | PSD失败降为UNVERIFIED并阻断Release profile |
| F2S-ADR-ACTION-001 | F2S-FR-SPEC-002、F2S-FR-GAME-001 | ActionRegistry/MotionSpec | F2S-TST-091 | F2S-DEV-M06-001 / F2S-EVD-M06-001 | 动作集变化须schema迁移、内容/测试/估算变更 |
| F2S-ADR-AST-001 | F2S-FR-PLAN-001、F2S-FR-PLAN-003 | AssetSpec/RepresentationPlan | F2S-TST-104、F2S-TST-105 | F2S-DEV-M06-003 / F2S-EVD-M06-003 | 规则版本可回滚；已批准素材不原位改写 |
| F2S-ADR-WEAPON-001 | F2S-FR-SPEC-001、F2S-FR-GAME-001 | WeaponSpec | F2S-TST-098 | F2S-DEV-M06-002 / F2S-EVD-M06-002 | 项目级WeaponSpec冻结后替换placeholder |
| F2S-ADR-GPU-001 | F2S-FR-RGPU-001、F2S-FR-COMP-001 | ComputeProvider/RemoteProfile | F2S-TST-RGPU-001、F2S-TST-RGPU-003 | F2S-DEV-M09-002 / F2S-EVD-M09-002 | 关闭远端/Worker，Core手工链继续 |
| F2S-ADR-SANDBOX-001 | F2S-NFR-SEC-005 | F2S-SEC-SANDBOX-001 | F2S-TST-RGPU-005、F2S-TST-E2E-SEC-005 | F2S-DEV-M09-001 / F2S-EVD-M09-001 | 探针失败物理移除Worker Pack |
| F2S-ADR-STO-001 | F2S-FR-PROJ-003、F2S-FR-PROJ-006 | F2S-SCH-001、F2S-SCH-002、F2S-SCH-003、F2S-SCH-004、F2S-SCH-005、F2S-SCH-006、F2S-SCH-007、F2S-SCH-008、F2S-SCH-009、F2S-IFC-401、F2S-IFC-402、F2S-IFC-403、F2S-IFC-404 | F2S-TST-111、F2S-TST-116 | F2S-DEV-M02-006 / F2S-EVD-M02-006 | 旧reader+只读备份+链式迁移 |
| F2S-ADR-JOB-001 | F2S-FR-COMP-003、F2S-NFR-REL-005 | JobExecutionRecord/JobOutputArtifact | F2S-TST-095、F2S-TST-IPC-004 | F2S-DEV-M02-004 / F2S-EVD-M02-004 | 旧Job schema链式迁移；失败输出不注册 |
| F2S-ADR-LIC-001 | F2S-NFR-LIC-001、F2S-NFR-LIC-002 | F2S-LIC-POLICY-001/SBOM | F2S-TST-E2E-LIC-001、F2S-TST-E2E-LIC-002 | F2S-DEV-M09-004 / F2S-EVD-M09-004 | 未知/禁止项物理移除，不能waive |
| F2S-ADR-WAIVER-001 | F2S-SCOPE-001、F2S-GOV-002、F2S-NFR-LIC-002 | 固定ActorAttestationPolicy+SignatureAlgorithmRegistry、publisher-signed OrganizationTrustEnrollment、ActorRegistry empty-genesis→原子enroll、WaiverProposal+三credential attestation、ActorTrust/RevocationBundle、OrganizationBundleFreshnessDeclaration+signingAttemptId/900s、Binding/Validation、ClockRecovery boundary、ReleaseGateCoverageReportV1 | F2S-TST-096、F2S-TST-099、F2S-TST-E2E-LIC-002 | F2S-DEV-M11-004 / F2S-EVD-M11-004 | 算法/编码不规范、无/过期/错edition enrollment、同ProjectId替换、partial bootstrap、错proposal/bundle rollback/旧epoch、含waiver但缺freshness声明、旧attempt/作业重启/负差/>900s、删required approval均阻断；P0/P1/硬类别不可binding |
| F2S-ADR-HITL-001 | F2S-FR-LAYER-006、F2S-FR-RIG-008、F2S-FR-GAME-002 | GateReport/decision-free resolver/DependencySnapshot/PrioritySlice+Observation/Approval registries/ReleaseGateCoverage | F2S-TST-090、F2S-TST-092、F2S-TST-093、F2S-TST-096 | F2S-DEV-M02-002、F2S-DEV-M03-005、F2S-DEV-M07-006 / 同号EVD | 关联stable inputs/slice匹配；coverage缺target或inactive即拒绝，历史不改 |
| F2S-ADR-REL-001 | F2S-FR-APP-001、F2S-FR-APP-002、F2S-FR-APP-003 | launcher/release manifest/update schema | F2S-TST-070、F2S-TST-071 | F2S-DEV-M10-002 / F2S-EVD-M10-002 | 回滚上一签名版本并保留项目reader |

### 2.2 R1 旧ID迁移

R1曾使用下列非canonical别名。它们只用于审计旧快照，状态均为`deprecated`，不得在R2、schema或原子任务中继续引用：

| Deprecated ID | Canonical ID |
| --- | --- |
| F2S-ADR-PLT-001 | F2S-ADR-PLAT-001 |
| F2S-ADR-INP-001 | F2S-ADR-INPUT-001 |
| F2S-ADR-ACT-001 | F2S-ADR-ACTION-001 |
| F2S-ADR-WPN-001 | F2S-ADR-WEAPON-001 |
| F2S-ADR-NET-001 | F2S-ADR-GPU-001 |
| F2S-ADR-REV-001 | F2S-ADR-HITL-001 |
| F2S-ADR-SPN-002 | F2S-ADR-TARGET-001 |
| F2S-OQ-DATA-001 | F2S-OQ-DATASET-001 |

其他文件不得重新定义这些canonical ADR，只能写“依据F2S-ADR-…”。

## 3. 已拒绝方案

| ADR ID | 方案 | 拒绝理由 |
| --- | --- | --- |
| F2S-ADR-REJ-001 | 单图一次性自动生成完整重度动作Spine | 信息缺失且无法承诺人体、视角、关键姿势质量 |
| F2S-ADR-REJ-002 | 前端直接访问文件/网络/CLI | 扩大WebView攻击面且无法统一审计 |
| F2S-ADR-REJ-003 | 每个动作独立text-to-image后拆层 | 身份、比例、层边界和pivot漂移 |
| F2S-ADR-REJ-004 | 把SQLite/redb缓存作为唯一项目源 | 缓存损坏会导致源资产丢失且难以人工审计 |
| F2S-ADR-REJ-005 | 捆绑Spine Runtime进行预览 | 与当前许可白名单和开发工具许可边界冲突 |
| F2S-ADR-REJ-006 | 使用 `latest` Spine CLI版本 | 发布不可复现，Editor/数据兼容可能漂移 |
| F2S-ADR-REJ-007 | 预选PyInstaller、Nuitka或任一Python冻结器作为默认商业打包器 | 工具/例外/插件/输出许可必须锁版逐项审计；Nuitka当前compiler为AGPL-3.0且runtime exception不改变compiler许可，故只能做fail-closed Spike，未通过即不发布Worker Pack |
| F2S-ADR-REJ-008 | AI自动批准hit frame/hitbox/cancel window | 玩法真值需要策划/动画师负责 |

## 4. 未决外部事项

| OQ ID | 未决事项 | 默认处理 | 何时阻塞 |
| --- | --- | --- | --- |
| F2S-OQ-SPN-001 | 本机未检测到合法 Spine Professional/Enterprise | 实现Adapter与静态测试；结果标UNVERIFIED | F2S-MILESTONE-M08的F2S-EVD-M08-007官方往返与最终“已验证”声明 |
| F2S-OQ-LGL-001 | Esoteric对闭源AI生产工具调用用户CLI的书面意见 | 不捆绑任何官方组件并给用户许可提示 | 商业公开发布 |
| F2S-OQ-SIGN-001 | Windows代码签名证书 | 生成未签名内部安装包并标警告 | 对外可信发布/SmartScreen体验 |
| F2S-OQ-GPU-001 | 私有远程GPU地址、证书与凭据 | Adapter关闭，以mock/本机协议验证 | 真实远端端到端验证 |
| F2S-OQ-WPN-001 | 单武器具体类型 | 使用WeaponSpec/primary_weapon | 特定攻击美术和提示词最终验收 |
| F2S-OQ-DATASET-001 | 用户不能提供可公开复用的完整验收素材集 | 工程只使用自制/采购且许可明确的合成fixture；用户素材仅本地验收 | F2S-DEV-M00-003产出fixture清单、许可与hash后关闭；在真实授权目标域样本存在前，阻断对真实目标域成功率的统计承诺 |
| F2S-OQ-BRAND-001 | 商业产品名称、商标和签名主体 | 工程代号FlashToSpine Production Assist | 安装器品牌和公开发布 |
| F2S-OQ-ORG-001 | 发布方签名的per-organization enrollment pack、组织offline root及三份独立审批credential尚未提供 | ActorRegistry保持sequence=0 empty genesis，governed P2 waiver禁用；修复全部finding的正常发布路径继续 | 只有在项目需要用governed waiver签名发布时阻塞；不得伪造pack、root或三签 |

## 5. 假设台账

已关闭事项：`F2S-OQ-TARGET-001` 于 2026-07-11 关闭；处置为接受 `F2S-ADR-TARGET-001`，V1 仅以 Spine Editor 4.2.43 为首个集成目标。

- `F2S-ASM-001`：V1首个外部集成目标为Spine Editor而非游戏引擎。
- `F2S-ASM-002`：产品主要由技术美术、Spine动画师或具备基础动画知识的用户使用。
- `F2S-ASM-003`：Windows 11 x64、8GB NVIDIA GPU是本地AI降级档；核心编辑器无GPU模型也可使用。
- `F2S-ASM-004`：用户愿意为每个角色审批母版、分层、Rig和关键姿势，不要求无人值守发布。
- `F2S-ASM-005`：用户所称“MIT、Apache、BSD等”已由`F2S-LIC-POLICY-001`转化为精确allowlist；若法务改变该policy，Python Runtime Pack、安装器和相关依赖必须重新审计。

假设一旦被实测或用户决策推翻，必须修改 canonical 文档、追踪矩阵和所有受影响任务，不能仅在本文件加备注。
