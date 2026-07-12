---
doc_id: F2S-DOC-DEVPLAN-M09-001
revision: 1.1
status: reviewed
canonical_for: [F2S-DEVPLAN-M09-CARDS-001, F2S-WU-M09]
depends_on: [F2S-DOC-DEVPLAN-M01-001, F2S-DOC-DEVPLAN-M02-001, F2S-DOC-DEVPLAN-M04-001, F2S-DOC-DEVPLAN-M08-001]
review_score_ref: F2S-SCORE-DEVPLAN-M09-001-R1
last_verified: 2026-07-11
---

# M09 安全、AI、私有远程 GPU 与质量原子开发计划

## 0. 里程碑合同

- Desktop Core 在无 Python、CUDA、模型、GPU、Spine 与网络时仍须完成手工 Production Assist 主链；这些组件不得进入 Core 安装包。
- Local Worker/Model Pack 是可选、可审计、独立签名和独立卸载的包。D2 只允许唯一 SandboxProfile=windows-appcontainer-v1；任一强控制失败即不发布 Worker Pack。
- 私有远程 GPU 默认关闭，只能连接用户明确配置并批准的私有 HTTPS 端点。凭据只保存为 Windows Credential Manager 引用；项目、日志和证据不得含 token。
- 发布依赖必须通过 F2S-LIC-POLICY-001；未知、未审计或 allowlist 外组件 fail closed。Spine Runtime/Editor、激活材料、CPython/CUDA/model 不得混入 Core。
- 真实 GPU、私有端点、外部凭据或法务意见不存在时，相关项写 NOT_RUN/EXTERNAL；mock、合成密码学或静态扫描不能替代真实声明。
- 公共命令只使用 npm run format:check、npm run lint、npm run typecheck、npm test、npm run test:integration、npm run build:core、npm run build:ai-pack、npm run release:verify。
- 全部执行证据严格消费 F2S-DEV-M02-001 拥有的 `schemas/src/evidence.schema.json` 唯一 `EvidenceEnvelope`；NOT_RUN/EXTERNAL 必须写明 `notRunReason` 与 `externalBlockers`。

## F2S-DEV-M09-001 | windows-appcontainer-v1隔离

1. **任务头**：目标是为可选本地 Worker 建立 windows-appcontainer-v1 强隔离、能力探针与发布裁决；非目标是把普通子进程、venv 或防火墙提示称为 sandbox。估算 4.5 人日。
2. **追踪**：F2S-FR-RGPU-001、F2S-NFR-PRIV-001、F2S-NFR-SEC-002、F2S-NFR-SEC-005、F2S-R3A-B12-P2-001；测试 F2S-TST-RGPU-005、F2S-TST-E2E-SEC-002、F2S-TST-E2E-SEC-005、F2S-TST-077；上游 F2S-DEV-M00-004、F2S-DEV-M01-001、F2S-DEV-M02-005、F2S-DEV-M04-004；下游 F2S-DEV-M09-003、F2S-DEV-M09-005、F2S-DEV-M10-001；P0，B12 primary owner。
3. **输入**：SandboxProfile registry、Worker protocol、job sandbox root、精确 Python/CUDA/model pack manifest（仅条件输入）、Windows 目标矩阵。
4. **输出**：crates/adapters/windows-appcontainer/src/lib.rs、crates/adapters/windows-appcontainer/src/probes.rs、config/sandbox/windows-appcontainer-v1.json、tests/security/appcontainer/**；运行时 SandboxAttestation 与 capability-probe report。
5. **设计边界**：AppContainer identity、最小 ACL、无继承网络、无项目根/用户目录/凭据访问、只读输入和专用输出；Core 不执行任意 Python；profile 不能降级为 unrestricted。
6. **work units**：

##### F2S-WU-M09-001-01 — AppContainer启动与ACL

   - output：AppContainer profile、SID/ACL materializer 与受控 process launcher。
   - reads：F2S-DEV-M02-005 WorkerPort、F2S-DEV-M04-004 job sandbox、F2S-SEC-SANDBOX-001、Windows capability baseline。
   - writes：crates/adapters/windows-appcontainer/src/lib.rs、config/sandbox/windows-appcontainer-v1.json。
   - steps：创建/查找 profile identity；为 operation root 赋最小 ACL；构造受限 token/process；只映射 input/output/log channel；默认无网络；退出后回收 lease。
   - command：npm run build:core。
   - tests：正常 echo worker、项目根读取、用户目录读取、Credential Manager、任意子进程、未授权网络。
   - evidence：evidence/M09/F2S-DEV-M09-001/F2S-WU-M09-001-01/launcher-tests.log、sandbox-profile-hash.json。
   - dependsOn：[F2S-DEV-M01-001, F2S-DEV-M02-005, F2S-DEV-M04-004]。
   - parallelSafety：sequential。
   - rollback：移除 Worker Pack 发布资格并禁用 D2；Core 手工链保持可用。
   - estimate：1.5d。

##### F2S-WU-M09-001-02 — 隔离逃逸与外联探针

   - output：escape/egress/ACL/symlink/handle inheritance 负向探针。
   - reads：F2S-WU-M09-001-01 launcher、恶意 synthetic worker。
   - writes：crates/adapters/windows-appcontainer/src/probes.rs、tests/security/appcontainer/escape.spec.ts。
   - steps：尝试父目录/junction/UNC/设备路径逃逸、句柄继承、命名管道滥用、loopback/公网 egress、进程树逃逸；每个探针产生稳定 ID 与原始退出裁决。
   - command：npm run test:integration。
   - tests：F2S-TST-E2E-SEC-005、F2S-TST-077；所有攻击必须被 OS 强控制拒绝。
   - evidence：evidence/M09/F2S-DEV-M09-001/F2S-WU-M09-001-02/security-probes.json、integration.log。
   - dependsOn：[F2S-WU-M09-001-01]。
   - parallelSafety：sequential。
   - rollback：失败即关闭 Worker capability，不添加软件层 allow-warning。
   - estimate：1.5d。

##### F2S-WU-M09-001-03 — PythonCUDA与GPU组合矩阵

   - output：Python/CUDA/GPU/model 组合矩阵与 B12 发布裁决。
   - reads：F2S-DEV-M00-004 spike、F2S-WU-M09-001-01、F2S-WU-M09-001-02、可选本地 pack 与 8GB GPU（若存在）。
   - writes：tests/security/appcontainer/python-cuda-gpu-matrix.spec.ts、fixtures/security/appcontainer/matrix-cases.json。
   - steps：至少覆盖无 Python、Python only、Python+model、Python+CUDA+GPU、驱动不兼容、GPU 不可见、模型越界；每格同时跑功能与强隔离探针；缺组件记录 NOT_RUN/EXTERNAL；只有全部 required probe PASS 才标 Worker Pack eligible。
   - command：npm run release:verify。
   - tests：F2S-TST-RGPU-005 与 F2S-R3A-B12-P2-001 exact matrix。
   - evidence：evidence/M09/F2S-DEV-M09-001/F2S-WU-M09-001-03/evidence.json、python-cuda-gpu-matrix.json；缺硬件/pack 的格子含 blockedClaim。
   - dependsOn：[F2S-DEV-M00-004, F2S-WU-M09-001-02]。
   - parallelSafety：sequential。
   - rollback：组合任一 required probe 失败即从发布清单物理移除 Worker/Model Pack；不回退为非隔离执行。
   - estimate：1.5d。
7. **正向验收**：受控 synthetic worker 只能访问本 operation 根和允许 IPC；在实际存在的 Python/CUDA/GPU 组合中功能与强隔离同时通过。
8. **负向与故障验收**：任一逃逸、egress、继承句柄、用户目录读取、profile 漂移或 required matrix 未证实均使 Worker Pack 不可发布；Core 不受影响。
9. **证据**：evidence/M09/F2S-DEV-M09-001/evidence.json（逻辑 ID F2S-EVD-M09-001） 必须引用 B12 exact matrix、OS build/profile hash 与逐探针结果；缺真实组件为 NOT_RUN/EXTERNAL。
10. **回滚**：禁用 D2 并从 pack manifest 移除 Worker；不得静默转为普通进程。
11. **完成定义**：3 WU、强隔离攻击面、B12 组合矩阵、发布 fail-closed 与同号 EVD 完整。
12. **退出状态**：DONE 可表示 adapter/probe 完成；只有实际目标组合全过才可 WorkerPackEligible=true。

## F2S-DEV-M09-002 | 私有远程GPU

1. **任务头**：目标是实现用户配置的私有远程 GPU Provider、显式传输审批和可验证收据；非目标是接入第三方公网 SaaS、生图服务或自动上传。估算 3.0 人日。
2. **追踪**：F2S-FR-RGPU-001、F2S-FR-RGPU-002、F2S-FR-RGPU-003、F2S-FR-RGPU-004、F2S-NFR-SEC-002、F2S-NFR-SEC-004、F2S-TST-CONTRACT-003、F2S-TST-RGPU-001、F2S-TST-RGPU-002、F2S-TST-RGPU-003、F2S-TST-RGPU-004；上游 F2S-DEV-M02-005、F2S-DEV-M04-004、F2S-DEV-M08-008；下游 F2S-DEV-M09-003、F2S-DEV-M09-007；P0。
3. **输入**：用户私有 endpoint、TLS policy、Credential Manager secret reference、job input manifest、per-operation network/transfer approval、retention policy。
4. **输出**：crates/adapters/private-remote-gpu/src/lib.rs、crates/adapters/private-remote-gpu/src/receipt.rs、tests/remote-gpu/provider-contract.spec.ts；运行时 RemoteTransferPlan、RemoteExecutionReceipt、deletion receipt。
5. **设计边界**：只允许 HTTPS 与 endpoint allowlist；禁止将 endpoint/token 写入可分享项目；只发送 manifest 中逐项批准的数据；Provider 返回均做 schema/size/hash 验证。
6. **work units**：

##### F2S-WU-M09-002-01 — 私有端点与传输计划

   - output：Provider adapter、TLS/endpoint/credential boundary 与 dry-run transfer plan。
   - reads：ProviderPort、CredentialRef、approved job input manifest。
   - writes：crates/adapters/private-remote-gpu/src/lib.rs。
   - steps：canonicalize endpoint；拒绝 HTTP、重定向换域、用户信息和非允许端口；从 OS store 取临时 token；展示逐文件 hash/bytes/purpose；无批准不建连。
   - command：npm run build:core。
   - tests：TLS 错误、重定向、DNS/endpoint 变化、token 缺失、未批准传输、超限响应。
   - evidence：evidence/M09/F2S-DEV-M09-002/F2S-WU-M09-002-01/provider-contract.log、transfer-plan-fixtures.json。
   - dependsOn：[F2S-DEV-M02-005, F2S-DEV-M04-004, F2S-DEV-M08-008]。
   - parallelSafety：sequential。
   - rollback：关闭 provider 配置并撤销 CredentialRef；不删除 OS 外部系统中的真实资源。
   - estimate：1.5d。

##### F2S-WU-M09-002-02 — 远端作业与删除收据

   - output：上传/状态/取消/下载/删除收据状态机与 mock/真实条件测试。
   - reads：F2S-WU-M09-002-01、private endpoint contract。
   - writes：crates/adapters/private-remote-gpu/src/receipt.rs、tests/remote-gpu/provider-contract.spec.ts。
   - steps：绑定 requestId/inputHash/endpoint identity；幂等轮询与取消；下载到 job sandbox 并校验 hash/schema；请求删除并记录 receipt；断线可重试但不重复提交未知作业。
   - command：npm run test:integration。
   - tests：F2S-TST-CONTRACT-003、F2S-TST-RGPU-001、F2S-TST-RGPU-002、F2S-TST-RGPU-003、F2S-TST-RGPU-004；重复、乱序、伪 hash、超大响应、取消竞态。
   - evidence：evidence/M09/F2S-DEV-M09-002/F2S-WU-M09-002-02/evidence.json、mock-receipts.json；真实端点缺失时 remote-e2e=NOT_RUN/EXTERNAL。
   - dependsOn：[F2S-WU-M09-002-01]。
   - parallelSafety：sequential。
   - rollback：保留本地 journal 供人工清理；禁用远端，不把未验证返回合入项目。
   - estimate：1.5d。
7. **正向验收**：mock 合同完整；若真实私有端点存在，逐项批准、TLS、执行、hash 校验、取消/删除与收据闭环。
8. **负向与故障验收**：远端默认关闭时代理观测项目数据/提示词/遥测外发为 0；错证书、换域、未批准、token 泄露、伪响应均 fail closed。
9. **证据**：evidence/M09/F2S-DEV-M09-002/evidence.json（逻辑 ID F2S-EVD-M09-002） 区分 mock PASS 与 realEndpoint NOT_RUN/EXTERNAL，不能合并成真实验证。
10. **回滚**：禁用 provider/撤销 credential reference；已提交本地 revision 不受远端失败影响。
11. **完成定义**：合同、传输批准、收据、删除、隐私负测完成；无第三方云默认配置。
12. **退出状态**：DONE 可在 mock 合同通过时成立；真实私有 GPU 能力仍以外部 evidence 独立表述。

## F2S-DEV-M09-003 | 威胁与负向测试

1. **任务头**：目标是把文件、IPC、Worker、Provider、CLI、WebView 与更新边界转成可重复攻击测试；非目标是一次性人工安全检查或以扫描器零告警替代设计证明。估算 4.5 人日。
2. **追踪**：F2S-FR-RGPU-002、F2S-FR-RGPU-003、F2S-FR-RGPU-004、F2S-NFR-PRIV-001、F2S-NFR-SEC-001、F2S-NFR-SEC-003、F2S-NFR-SEC-004、F2S-NFR-SEC-005；测试 F2S-TST-E2E-SEC-001、F2S-TST-E2E-SEC-003、F2S-TST-E2E-SEC-004、F2S-TST-E2E-SEC-005、F2S-TST-E2E-SEC-006、F2S-TST-062、F2S-TST-109、F2S-TST-IPC-002、F2S-TST-IPC-006、F2S-TST-IPC-007；上游 F2S-DEV-M01-003、F2S-DEV-M02-001、F2S-DEV-M02-005、F2S-DEV-M02-006、F2S-DEV-M03-002、F2S-DEV-M03-003、F2S-DEV-M04-004、F2S-DEV-M08-005、F2S-DEV-M08-006、F2S-DEV-M09-001、F2S-DEV-M09-002；下游 F2S-DEV-M10-002、F2S-DEV-M11-004；P0。
3. **输入**：threat model、所有边界 schema/size/path policy、恶意 fixtures、sandbox/provider/CLI adapters。
4. **输出**：tests/security/adversarial/**、tools/security/run-adversarial.ps1、tools/security/attack-catalog.json；运行时 machine-readable attack report。
5. **设计边界**：测试只作用于合成 fixture 与隔离测试根；不攻击外部服务、不扫描用户目录、不记录秘密；安全失败不可 waiver。
6. **work units**：

##### F2S-WU-M09-003-01 — 文件边界攻击集

   - output：文件/路径/解析/解压/图片/manifest 攻击集。
   - reads：F2S-DEV-M02-001 schema、F2S-DEV-M03-002 与 F2S-DEV-M03-003 import limits、F2S-DEV-M02-006 storage roots、F2S-DEV-M08-005 export/publish。
   - writes：tests/security/adversarial/file-boundary.spec.ts、fixtures/security/files/**。
   - steps：覆盖 traversal、junction/symlink、UNC/device/ADS、TOCTOU、zip bomb、像素/压缩边界、polyglot、重复键、深层 JSON、hash mismatch、publish 越界。
   - command：npm run test:integration。
   - tests：F2S-TST-E2E-SEC-001、F2S-TST-109。
   - evidence：evidence/M09/F2S-DEV-M09-003/F2S-WU-M09-003-01/file-attacks.json、integration.log。
   - dependsOn：[F2S-DEV-M02-001, F2S-DEV-M02-006, F2S-DEV-M03-002, F2S-DEV-M03-003, F2S-DEV-M08-005]。
   - parallelSafety：shared-lock:m09-003。
   - rollback：删除仅测试 fixture；生产拒绝策略不得因测试不便放宽。
   - estimate：1.5d。

##### F2S-WU-M09-003-02 — 进程与IPC攻击集

   - output：IPC/WebView/CLI/Worker 攻击集。
   - reads：IPC allowlist、WebView CSP、Spine CLI policy、AppContainer probes。
   - writes：tests/security/adversarial/process-boundary.spec.ts、fixtures/security/process/**。
   - steps：覆盖未知 command、超限 frame、重放/乱序/取消竞态、通用 shell/fs/http、参数注入、stdout bomb、handle inheritance、sandbox escape/egress。
   - command：npm run test:integration。
   - tests：F2S-TST-E2E-SEC-003、F2S-TST-E2E-SEC-005、F2S-TST-E2E-SEC-006、F2S-TST-062、F2S-TST-IPC-002、F2S-TST-IPC-006、F2S-TST-IPC-007。
   - evidence：evidence/M09/F2S-DEV-M09-003/F2S-WU-M09-003-02/process-attacks.json。
   - dependsOn：[F2S-DEV-M01-003, F2S-DEV-M08-006, F2S-DEV-M09-001]。
   - parallelSafety：sequential。
   - rollback：禁用受影响 capability；不添加危险 debug bridge。
   - estimate：1.5d。

##### F2S-WU-M09-003-03 — 远端凭据与隐私攻击集

   - output：Provider/credential/privacy 攻击集与统一 attack catalog。
   - reads：F2S-DEV-M09-002 provider、credential/redaction policies。
   - writes：tests/security/adversarial/remote-boundary.spec.ts、tools/security/run-adversarial.ps1、tools/security/attack-catalog.json。
   - steps：覆盖 TLS/redirect/SSRF、credential leak、未批准外发、恶意返回、取消重放、日志泄露；每攻击映射 requirement/test/finding severity。
   - command：npm run release:verify。
   - tests：F2S-TST-E2E-SEC-004、F2S-TST-E2E-PRIV-001、F2S-TST-E2E-PRIV-002、F2S-TST-109。
   - evidence：evidence/M09/F2S-DEV-M09-003/F2S-WU-M09-003-03/evidence.json、attack-catalog-results.json。
   - dependsOn：[F2S-DEV-M09-002, F2S-WU-M09-003-01, F2S-WU-M09-003-02]。
   - parallelSafety：shared-lock:m09-003。
   - rollback：关闭 remote provider 并保留 finding；不得降级 severity 绕过发布。
   - estimate：1.5d。
7. **正向验收**：attack catalog 每项有边界、期望拒绝码、test ID 与 evidence；全量运行不触碰真实用户/外部系统。
8. **负向与故障验收**：任一攻击成功、崩溃、数据泄漏、无界资源或结果缺证据均是发布阻断；P0/P1 与安全类别不可 waiver。
9. **证据**：evidence/M09/F2S-DEV-M09-003/evidence.json（逻辑 ID F2S-EVD-M09-003） 汇总三个攻击域、commit/toolchain/OS hash 和 findings。
10. **回滚**：先禁用有漏洞的 capability，再修复；不可删除失败 fixture 或把 expected fail 当 PASS。
11. **完成定义**：3 WU、所有 exact 安全测试映射、攻击目录与同号 EVD 完成。
12. **退出状态**：DONE 要求 required 攻击全部被拒绝；真实外部渗透测试仍可另列 EXTERNAL，不冒充已完成。

## F2S-DEV-M09-004 | 许可/SBOM/Model BOM

1. **任务头**：目标是对源码依赖、二进制、模型、字体、图标、样例资产生成可审计清单并以 allowlist 阻断发布；非目标是由工程团队提供法律保证。估算 3.0 人日。
2. **追踪**：F2S-NFR-LIC-001、F2S-NFR-LIC-002、F2S-ADR-LIC-001、F2S-ADR-SPN-003；测试 F2S-TST-E2E-LIC-001、F2S-TST-E2E-LIC-002、F2S-TST-065、F2S-TST-078；上游 F2S-DEV-M00-002、F2S-DEV-M01-002、F2S-DEV-M09-001；下游 F2S-DEV-M10-001、F2S-DEV-M10-006、F2S-DEV-M11-004；P0。
3. **输入**：F2S-LIC-POLICY-001、锁文件、构建图、pack manifests、asset/model source manifests、外部法务结论（若存在）。
4. **输出**：policy/licensing/f2s-license-policy-v1.json、tools/compliance/inventory.ps1、tools/compliance/verify-policy.ps1、tests/compliance/license-gate.spec.ts；执行期输出 SBOM、THIRD_PARTY_NOTICES、ModelBOM 与 provenance index。
5. **设计边界**：只接受 canonical allowlist；未知/来源不明/策略外 fail closed；代码许可证不推导模型权重或输出权利；Core/Worker/Model Pack 分开清单；不得捆绑 Spine 官方组件。
6. **work units**：

##### F2S-WU-M09-004-01 — 许可策略与组件清单

   - output：许可策略机与跨生态 inventory generator。
   - reads：Cargo/npm lock、asset/model manifests、F2S-LIC-POLICY-001。
   - writes：policy/licensing/f2s-license-policy-v1.json、tools/compliance/inventory.ps1。
   - steps：按 package/file/asset/model 枚举版本、来源、hash、SPDX/结论；区分 Core/Worker/Model Pack/开发工具；生成稳定排序 SBOM/ModelBOM/notices 输入。
   - command：npm run build:ai-pack。
   - tests：MIT/Apache/BSD 等 allowlist fixture、unknown、双许可、缺 source/hash、模型仅有代码许可。
   - evidence：evidence/M09/F2S-DEV-M09-004/F2S-WU-M09-004-01/inventory-fixtures.json、build-inventories.log。
   - dependsOn：[F2S-DEV-M00-002, F2S-DEV-M01-002, F2S-DEV-M09-001]。
   - parallelSafety：shared-lock:m09-004。
   - rollback：恢复上一签名 policy snapshot；新依赖保持不可发布。
   - estimate：1.5d。

##### F2S-WU-M09-004-02 — 发布许可硬门

   - output：发布策略验证器、禁止组件扫描与许可负测。
   - reads：F2S-WU-M09-004-01 inventory、构建 staging、policy hash。
   - writes：tools/compliance/verify-policy.ps1、tests/compliance/license-gate.spec.ts。
   - steps：拒绝 unknown/disallowed/missing；扫描 Spine Runtime/Editor/activation、CPython/CUDA/model 混入 Core；检查每 pack SBOM/notices/BOM 完整；策略变化须 ADR/法务证据。
   - command：npm run release:verify。
   - tests：F2S-TST-E2E-LIC-001、F2S-TST-E2E-LIC-002、F2S-TST-065、F2S-TST-078；注入禁用包、未知许可证与假 Runtime 文件。
   - evidence：evidence/M09/F2S-DEV-M09-004/F2S-WU-M09-004-02/evidence.json、sbom-validation.json、forbidden-component-scan.json；法务意见缺失标 EXTERNAL。
   - dependsOn：[F2S-WU-M09-004-01]。
   - parallelSafety：shared-lock:m09-004。
   - rollback：从发布 manifest 物理移除失败组件/pack；不使用 waiver 放行硬许可类别。
   - estimate：1.5d。
7. **正向验收**：每个发布文件反查 source/version/hash/license/pack；Core 与可选 pack 清单分离且稳定。
8. **负向与故障验收**：unknown、allowlist 外、缺 source/hash、模型权利不明、Spine Runtime/Editor/activation 或 AI 组件混入 Core 均使 release:verify 失败。
9. **证据**：evidence/M09/F2S-DEV-M09-004/evidence.json（逻辑 ID F2S-EVD-M09-004） 与实际 inventory hash 绑定；商业法务保证缺失为 NOT_RUN/EXTERNAL，不影响保守工程门但阻断对应声明。
10. **回滚**：移除新增依赖/资产/pack 或恢复上一政策；禁止在单依赖处静默 override。
11. **完成定义**：两 WU、正负许可 fixture、禁止组件扫描、SBOM/ModelBOM/notices 生成与同号 EVD 完成。
12. **退出状态**：DONE 表示工程 allowlist 门完成，不等于外部律师已出具商业意见。

## F2S-DEV-M09-005 | 性能验证

1. **任务头**：目标是执行七项 NFR 性能预算以及 Pixi/WebView2/DPI/context-loss 证据；非目标是通过降源分辨率、静默换模型或隐藏诊断满足指标。估算 3.0 人日。
2. **追踪**：F2S-NFR-PERF-001、F2S-NFR-PERF-002、F2S-NFR-PERF-003、F2S-NFR-PERF-004、F2S-NFR-PERF-005、F2S-NFR-PERF-006、F2S-NFR-PERF-007、F2S-R3A-B08-P2-001；测试 F2S-TST-E2E-PERF-001、F2S-TST-E2E-PERF-002、F2S-TST-E2E-PERF-003、F2S-TST-E2E-PERF-004、F2S-TST-E2E-PERF-005、F2S-TST-E2E-PERF-006、F2S-TST-E2E-PERF-007、F2S-TST-074、F2S-TST-086、F2S-TST-087；上游 F2S-DEV-M01-003、F2S-DEV-M04-006、F2S-DEV-M07-003、F2S-DEV-M09-001；下游 F2S-DEV-M10-005、F2S-DEV-M11-004；P0，B08 primary owner。
3. **输入**：F2S-ENV-WIN-001、F2S-FIX-PROJ-001、8GB GPU（若存在）、cold/warm run protocol，以及 F2S-DEV-M07-003/F2S-EVD-M07-003 唯一只读的 `evidence/M07/F2S-DEV-M07-003/performance-fixture-contract.json`、fixture manifest、profile 与 raw sample files。
4. **输出**：tools/benchmark/run-performance.ps1、tests/performance/performance-budget.spec.ts、tests/performance/renderer-resilience.spec.ts、fixtures/performance/scenarios.json；执行期性能报告与 traces。
5. **设计边界**：每项独立判定；记录 OS/CPU/GPU/driver/WebView2/DPI/resolution/commit/cache；AI 推理不承诺固定总耗时，只报告吞吐与峰值；代理资产与源资产类型分离。
6. **work units**：

##### F2S-WU-M09-005-01 — 核心性能预算

   - output：启动、输入延迟、FPS、长任务、容量与提交 benchmark harness。
   - reads：Desktop instrumentation、NFR thresholds、F2S-DEV-M07-003 的 evidence/M07/F2S-DEV-M07-003/performance-fixture-contract.json 与其中 fixtureManifestSha256、profileSha256、rawSampleFiles[path,sha256,count]、rendererBuildSha256、producer/consumer/capabilityState；禁止替换 fixture/profile/raw。
   - writes：tools/benchmark/run-performance.ps1、tests/performance/performance-budget.spec.ts。
   - steps：1) 先重算并逐项匹配 fixtureManifestSha256、profileSha256 和全部 rawSampleFiles hash/count；2) 强制沿用 10 秒预热+120 秒采样与 nearest-rank P95，不接受临时场景；3) 冷启动至少 30、热启动至少 100；4) 测可交互 P95≤10s、输入 P95≤100ms、1920×1080 基础≥60FPS/诊断≥30FPS、阻塞≤200ms/进度≤2s、manifest P95≤500ms；5) 同 fixture/profile/raw 任一漂移即 B08 FAILED；6) 软限提示、硬限拒绝。
   - command：npm run release:verify。
   - tests：F2S-R3A-B08-P2-001、F2S-TST-E2E-PERF-001、F2S-TST-E2E-PERF-002、F2S-TST-E2E-PERF-003、F2S-TST-E2E-PERF-004、F2S-TST-E2E-PERF-005、F2S-TST-E2E-PERF-007、F2S-TST-074、F2S-TST-086；含 manifest/profile/raw 任一 hash 漂移、预热/采样缩短、另建 fixture 的负例。
   - evidence：evidence/M09/F2S-DEV-M09-005/F2S-WU-M09-005-01/performance-summary.json、contract-consumption.json、raw-traces/**；contract-consumption.json 绑定上游 contract/fixture/profile/raw hash 与 10s+120s 协议。
   - dependsOn：[F2S-DEV-M01-003, F2S-DEV-M04-006, F2S-DEV-M07-003]。
   - parallelSafety：sequential。
   - rollback：恢复引入退化的变更；不得调低 canonical 阈值作为修复。
   - estimate：1.5d。

##### F2S-WU-M09-005-02 — B08渲染与8GB矩阵

   - output：B08 Pixi/WebView2/DPI/context-loss 与 8GB AI 预检矩阵。
   - reads：renderer、WebView2 exact version、DPI 100/125/150/200%、F2S-DEV-M09-001 Worker probe、8GB GPU，以及同一 F2S-DEV-M07-003 contract 的 fixtureManifestSha256、profileSha256、rawSampleFiles 与 10 秒预热+120 秒采样协议。
   - writes：tests/performance/renderer-resilience.spec.ts、fixtures/performance/scenarios.json。
   - steps：1) 重算上游 contract/fixture/profile/raw hash 并与 WU01 contract-consumption.json 相等；2) 按原 10s+120s 协议在每 DPI 测坐标/清晰度/延迟/FPS；3) 注入 WebGL context loss 并重建，最后 committed revision 不丢；4) 以 hash 漂移、缺 raw、样本 count 不符和临时 fixture 为 fail-closed 负例；5) 8GB 档预检 model/input/VRAM/RAM/temp，资源不足阻断或等待显式 tile/offload/CPU/较小模型选择并记录质量影响；6) 禁止静默降质。
   - command：npm run release:verify。
   - tests：F2S-TST-E2E-PERF-002、F2S-TST-E2E-PERF-003、F2S-TST-E2E-PERF-006、F2S-TST-086、F2S-TST-087、F2S-R3A-B08-P2-001。
   - evidence：evidence/M09/F2S-DEV-M09-005/F2S-WU-M09-005-02/evidence.json、b08-renderer-matrix.json、contract-consumption.json、gpu8gb-preflight.json；缺 GPU/pack 格为 NOT_RUN/EXTERNAL，但 renderer contract hash 校验仍必须执行。
   - dependsOn：[F2S-DEV-M07-003, F2S-DEV-M09-001]。
   - parallelSafety：sequential。
   - rollback：禁用未通过的 Worker/高级预览路径；Core 手工链与源资产保持。
   - estimate：1.5d。
7. **正向验收**：七项指标分别给出样本、P95/FPS/峰值与环境；B08 四维矩阵有逐格结果，context loss 后恢复且不丢 commit；上下游 fixtureManifestSha256/profileSha256/rawSampleFiles 与 10s+120s 协议逐项相等。
8. **负向与故障验收**：缺样本/环境、fixture/profile/raw 任一 hash 或 count 漂移、预热/采样缩短、临时场景替换、聚合掩盖单项失败、静默降质、真实硬件未跑却标 PASS、调低阈值均失败。
9. **证据**：evidence/M09/F2S-DEV-M09-005/evidence.json（逻辑 ID F2S-EVD-M09-005） 是 B08 关闭出口；必须引用两 WU 的 contract-consumption.json、上游 F2S-EVD-M07-003 与原始 trace/摘要 hash，外部硬件缺失明确 NOT_RUN/EXTERNAL。
10. **回滚**：回退退化 commit 或关闭可选能力；性能阈值变更必须新 ADR，不能在本任务内改。
11. **完成定义**：2 WU、七预算、B08、8GB 预检、context-loss 和同号 EVD 完成。
12. **退出状态**：DONE 只针对已实际执行环境；未覆盖 GPU/OS 组合不外推。

## F2S-DEV-M09-006 | 恢复故障注入

1. **任务头**：目标是验证 NTFS flush/replace、文件占用、kill、断电近似、崩溃恢复与 publish 重试；非目标是靠 happy-path 单测推断持久化可靠。估算 4.5 人日。
2. **追踪**：F2S-NFR-REL-002、F2S-NFR-REL-005、F2S-R3A-B11-P2-001；测试 F2S-TST-E2E-REL-002、F2S-TST-E2E-REL-005、F2S-TST-E2E-007、F2S-TST-064、F2S-TST-111、F2S-TST-IPC-005；上游 F2S-DEV-M02-006、F2S-DEV-M02-007、F2S-DEV-M08-005；下游 F2S-DEV-M10-005、F2S-DEV-M11-001、F2S-DEV-M11-004；P0，B11 primary owner。
3. **输入**：OperationJournal、CAS/project commit、export/publish state machines、fault-point registry、NTFS test volume。
4. **输出**：tools/fault-injection/run-ntfs-faults.ps1、tests/fault/ntfs-atomicity.spec.ts、tests/fault/recovery-ui.spec.ts、fixtures/fault/scenarios.json；运行时 kill matrix/recovery reports。
5. **设计边界**：只对专用合成测试根和受控子进程注入；不得 kill 当前开发 shell 或用户应用；恢复只选择完整旧/新版本，索引/缓存不是事实源。
6. **work units**：

##### F2S-WU-M09-006-01 — 受控故障注入Harness

   - output：稳定 fault-point 协议与受控子进程 kill harness。
   - reads：F2S-DEV-M02-006 commit/journal phases、F2S-DEV-M02-007 recovery phases、F2S-DEV-M08-005 export/publish phases。
   - writes：tools/fault-injection/run-ntfs-faults.ps1、fixtures/fault/scenarios.json。
   - steps：登记 write/flush/fsync-parent/replace/rename/index-update/publish-copy 等点；每次仅启动并终止测试 child；记录 seed、phase、PID、volume、expected invariant。
   - command：npm run test:integration。
   - tests：F2S-TST-064、F2S-TST-IPC-005；harness 自身越界保护。
   - evidence：evidence/M09/F2S-DEV-M09-006/F2S-WU-M09-006-01/fault-harness.json。
   - dependsOn：[F2S-DEV-M02-006, F2S-DEV-M02-007, F2S-DEV-M08-005]。
   - parallelSafety：sequential。
   - rollback：停止 harness 并删除专用临时根；不触碰真实项目。
   - estimate：1.5d。

##### F2S-WU-M09-006-02 — B11 NTFS原子矩阵

   - output：B11 flush/replace/占用/kill 矩阵。
   - reads：F2S-WU-M09-006-01、NTFS volume、atomic commit adapters。
   - writes：tests/fault/ntfs-atomicity.spec.ts。
   - steps：在每个阶段注入 kill；用共享/拒绝共享句柄占用源、目标、临时文件；验证 flush 后 replace、rename 失败、重启扫描；只允许完整旧或新版本，半写/丢确认 revision 为硬失败。
   - command：npm run test:integration。
   - tests：F2S-TST-E2E-REL-002、F2S-TST-E2E-007、F2S-TST-111、F2S-R3A-B11-P2-001。
   - evidence：evidence/M09/F2S-DEV-M09-006/F2S-WU-M09-006-02/b11-ntfs-matrix.json、ntfs-fault.log。
   - dependsOn：[F2S-WU-M09-006-01]。
   - parallelSafety：sequential。
   - rollback：隔离失败 artifact；保留最后完整 revision；不自动删除占用文件。
   - estimate：1.5d。

##### F2S-WU-M09-006-03 — 重启恢复与幂等重试

   - output：重启恢复选择、未完成 job/export/publish 展示与幂等重试测试。
   - reads：kill matrix、journal/quarantine、recovery UI contract。
   - writes：tests/fault/recovery-ui.spec.ts。
   - steps：重启后列出 last saved/autosave/incomplete；由用户选择，不自动合并；验证 remote/CLI/publish 不重复副作用，旧 snapshot 可重试；损坏索引可重建。
   - command：npm run release:verify。
   - tests：F2S-TST-E2E-REL-005、F2S-TST-E2E-REL-002、F2S-TST-064。
   - evidence：evidence/M09/F2S-DEV-M09-006/F2S-WU-M09-006-03/evidence.json、recovery-choice-report.json。
   - dependsOn：[F2S-WU-M09-006-02]。
   - parallelSafety：shared-lock:m09-006。
   - rollback：回到只读恢复模式并禁止写入；用户数据不删除。
   - estimate：1.5d。
7. **正向验收**：每个 NTFS 阶段 kill/占用后仅加载完整旧/新 revision；恢复 UI 展示三个来源和未完成副作用，由用户选择。
8. **负向与故障验收**：半写可见、旧 revision 被覆盖、自动采用未批准恢复、重复远端/CLI/publish、越界 kill 均为硬失败。
9. **证据**：evidence/M09/F2S-DEV-M09-006/evidence.json（逻辑 ID F2S-EVD-M09-006） 是 B11 关闭出口，含 volume/fs/OS、fault seed、phase、expected/actual hash。
10. **回滚**：关闭写能力进入只读恢复；保留 journal/quarantine 和最后完整版本。
11. **完成定义**：3 WU、B11 四类注入、恢复 UI、幂等副作用与同号 EVD 完成。
12. **退出状态**：DONE 需要真实 NTFS 运行证据；其他文件系统结果不得冒充 Windows 发布依据。

## F2S-DEV-M09-007 | 日志与诊断脱敏

1. **任务头**：目标是统一结构化事件、诊断包和默认脱敏；非目标是采集遥测、全文 prompt、图像或用户目录清单。估算 3.0 人日。
2. **追踪**：F2S-FR-PROJ-007、F2S-NFR-PRIV-002、F2S-NFR-OBS-001；测试 F2S-TST-RGPU-006、F2S-TST-E2E-PRIV-002、F2S-TST-E2E-OBS-001；上游 F2S-DEV-M01-005、F2S-DEV-M02-005、F2S-DEV-M02-008、F2S-DEV-M04-004、F2S-DEV-M08-008、F2S-DEV-M09-001、F2S-DEV-M09-002；下游 F2S-DEV-M10-005、F2S-DEV-M11-003；P0。
3. **输入**：event catalog、job/operation IDs、error taxonomy、redaction policy、各 adapter 的 safe diagnostic DTO。
4. **输出**：crates/adapters/diagnostics/src/lib.rs、crates/adapters/diagnostics/src/redaction.rs、tests/privacy/diagnostics-redaction.spec.ts；运行时 local logs 与 user-approved diagnostic bundle。
5. **设计边界**：默认本地、无自动上传；允许 eventId/severity/correlationId/stage/size/hash 前缀/稳定错误码；禁止 image bytes/full prompt/token/absolute path/username/hostname/license data。
6. **work units**：

##### F2S-WU-M09-007-01 — 结构化本地日志

   - output：结构化 logger adapter、字段 allowlist 与 bounded rotation。
   - reads：event catalog、application diagnostic port。
   - writes：crates/adapters/diagnostics/src/lib.rs。
   - steps：schema 校验 event；拒绝未知敏感字段；限制 message/stack/volume；轮转与用户清除；无 endpoint/telemetry transport。
   - command：npm run build:core。
   - tests：结构化 event/job/severity、超长字段、未知 key、rotation、离线零请求。
   - evidence：evidence/M09/F2S-DEV-M09-007/F2S-WU-M09-007-01/logger-tests.log、event-schema-report.json。
   - dependsOn：[F2S-DEV-M01-005, F2S-DEV-M02-005]。
   - parallelSafety：shared-lock:m09-007。
   - rollback：切换到最小 stderr/本地错误码模式；不上传历史日志。
   - estimate：1.5d。

##### F2S-WU-M09-007-02 — 诊断包脱敏

   - output：redaction pipeline、diagnostic bundle manifest 与 canary 扫描。
   - reads：各 adapter synthetic logs、secret/path/prompt/image canaries。
   - writes：crates/adapters/diagnostics/src/redaction.rs、tests/privacy/diagnostics-redaction.spec.ts。
   - steps：结构化字段级脱敏后再打包；扫描 token、绝对路径、用户名、主机、全文 prompt、PNG magic/像素；用户预览并显式选择保存，不发送。
   - command：npm run test:integration。
   - tests：F2S-TST-RGPU-006、F2S-TST-E2E-PRIV-002、F2S-TST-E2E-OBS-001。
   - evidence：evidence/M09/F2S-DEV-M09-007/F2S-WU-M09-007-02/evidence.json、redaction-canary-report.json。
   - dependsOn：[F2S-WU-M09-007-01, F2S-DEV-M09-001, F2S-DEV-M09-002]。
   - parallelSafety：shared-lock:m09-007。
   - rollback：禁用诊断包导出；保留可见的最小错误码。
   - estimate：1.5d。
7. **正向验收**：事件具稳定 ID/job/severity；用户可预览本地诊断包；必要 hash/版本/阶段可审计。
8. **负向与故障验收**：任何 image/full prompt/token/absolute path/username/hostname/许可信息 canary 出现即失败；默认网络请求必须为 0。
9. **证据**：evidence/M09/F2S-DEV-M09-007/evidence.json（逻辑 ID F2S-EVD-M09-007） 绑定 canary 字典版本和 bundle hash。
10. **回滚**：关闭 bundle 与 verbose log，使用最小本地错误码；不得以关闭脱敏换可诊断性。
11. **完成定义**：两 WU、结构化日志、bounded storage、隐私 canary 和离线检查完成。
12. **退出状态**：DONE 表示本地诊断可用且脱敏；不表示存在或启用遥测。

## 里程碑退出门

1. 7 个 DEV、17 个唯一 WU、7 个同号 EVD 出口均存在。
2. B12、B08、B11 分别由 F2S-DEV-M09-001、F2S-DEV-M09-005、F2S-DEV-M09-006 产生 exact test/evidence；未执行真实环境不得关闭。
3. Core 发布扫描中 Python、CUDA、model、Spine Runtime/Editor/activation 数量为 0。
4. Worker Pack 仅在 windows-appcontainer-v1 全部 required probes PASS 时 eligible；不存在软件降级路径。
5. 私有远程 GPU 默认关闭；真实 endpoint/credential 缺失时 NOT_RUN/EXTERNAL；mock PASS 不外推。
6. allowlist 外、未知许可、安全/隐私/数据完整性 finding 为硬阻断，不可 waiver。
7. M10 只能消费分包清单、签名输入、质量报告和结构化外部状态，不得把缺失证据改写为 PASS。
