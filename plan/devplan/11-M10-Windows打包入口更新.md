---
doc_id: F2S-DOC-DEVPLAN-M10-001
revision: 1.1
status: reviewed
canonical_for: [F2S-DEVPLAN-M10-CARDS-001, F2S-WU-M10]
depends_on: [F2S-DOC-DEVPLAN-M08-001, F2S-DOC-DEVPLAN-M09-001]
review_score_ref: F2S-SCORE-DEVPLAN-M10-001-R1
last_verified: 2026-07-11
---

# M10 Windows 打包、入口与更新原子开发计划

## 0. 里程碑合同

- 正式主程序为 FlashToSpine.exe；仓库根与安装后可双击入口为原生 FlashToSpineLauncher.exe，不得以 cmd、PowerShell、快捷方式脚本或动态下载器替代。
- Launcher 只定位、验签、启动主程序并通过随机 named pipe + nonce 健康握手；不得编译、下载、提权、改 ExecutionPolicy、调用 shell 或访问项目内容。
- Windows 11 x64 是 P0；Windows 10 22H2 x64 是 P1 尽力兼容。普通用户、中文/空格/长路径、100–200% DPI、系统 WebView2 均进入矩阵。
- Core、Worker Pack、Model Pack 分离；Core 不含 Python、CUDA、模型、Spine Runtime/Editor/激活信息。安装器使用 per-user NSIS zlib 与系统 Evergreen WebView2。
- 代码签名证书、可信 timestamp 与真实发布主体缺失时，可生成清晰标记的内部未签名候选；商业签名、SmartScreen 与公开更新声明必须为 NOT_RUN/EXTERNAL。
- 公共命令只使用 npm ci、npm run bootstrap:check、npm run format:check、npm run lint、npm run typecheck、npm test、npm run test:integration、npm run build:core、npm run build:ai-pack、npm run release:verify。
- 全部执行证据严格消费 F2S-DEV-M02-001 拥有的 `schemas/src/evidence.schema.json` 唯一 `EvidenceEnvelope`；签名、timestamp、SmartScreen 外部状态只能经该 envelope 表达。

## F2S-DEV-M10-001 | Windows打包

1. **任务头**：目标是生成可审计的 per-user Windows Core 安装包及独立可选 pack；非目标是静默安装系统运行时、把开发工具带入发行物或删除用户项目。估算 3.0 人日。
2. **追踪**：F2S-FR-APP-003、F2S-NFR-LIC-002、F2S-REL-EDITION-001、F2S-REL-ARTIFACT-001；测试 F2S-TST-E2E-008、F2S-TST-E2E-LIC-002、F2S-TST-076；上游 F2S-DEV-M08-007、F2S-DEV-M08-008、F2S-DEV-M09-001、F2S-DEV-M09-004；下游 F2S-DEV-M10-002、F2S-DEV-M10-005、F2S-DEV-M10-006、F2S-DEV-M11-001、F2S-DEV-M11-004；P0。
3. **输入**：Core build、可选 Worker/Model pack eligibility、SBOM/notices/BOM、精确 WebView2 policy、版本/渠道/publisher 占位元数据。
4. **输出**：installer/tauri/tauri.release.overlay.json、installer/nsis/core.nsi、installer/nsis/worker-pack.nsi、installer/nsis/model-pack.nsi、tests/packaging/package-layout.spec.ts；执行期只读合并 M01-003 拥有的 apps/desktop/src-tauri/tauri.conf.json，并输出 dist/windows/core/**、worker-pack/**、model-pack/**。
5. **设计边界**：per-user、无管理员需求；zlib 非 solid；系统 Evergreen WebView2 为前置诊断而非捆绑；项目目录永不属于卸载清单；pack 之间无隐式安装。
6. **work units**：

##### F2S-WU-M10-001-01 — Core安装布局

   - output：Core NSIS/Tauri packaging 配置与确定文件清单。
   - reads：F2S-DEV-M09-004 approved inventory、Core binaries、release layout contract。
   - writes：installer/tauri/tauri.release.overlay.json、installer/nsis/core.nsi。
   - steps：只读载入 M01-003 的 base tauri.conf；用发布 overlay 冻结 per-user install root；显式列 launcher/main/resources/licenses；检查系统 WebView2 并给稳定诊断；定义 install/repair/uninstall；排除用户项目和外部设置；合并结果仅写 build staging。
   - command：npm run release:verify。
   - tests：普通用户安装、修复、卸载；WebView2 缺失；项目保留；包内开发工具/秘密扫描。
   - evidence：evidence/M10/F2S-DEV-M10-001/F2S-WU-M10-001-01/core-package-layout.json、release-verify.log。
   - dependsOn：[F2S-DEV-M09-004]。
   - parallelSafety：shared-lock:m10-001。
   - rollback：保留上一版本安装包；新包不发布且不修改用户安装。
   - estimate：1.5d。

##### F2S-WU-M10-001-02 — 可选Pack分离

   - output：Worker/Model Pack 独立安装器、manifest 与 Core 禁止组件扫描。
   - reads：F2S-DEV-M09-001 WorkerPackEligible、F2S-DEV-M09-004 per-pack inventory、AI pack build。
   - writes：installer/nsis/worker-pack.nsi、installer/nsis/model-pack.nsi、tests/packaging/package-layout.spec.ts。
   - steps：仅 eligible 时 materialize pack；独立版本/hash/许可/卸载；验证 Core 中 Python/CUDA/model=0，全部包中 Spine Runtime/Editor/activation=0；pack 不可用时 Core 正常。
   - command：npm run release:verify。
   - tests：F2S-TST-E2E-LIC-002、F2S-TST-076；注入禁止文件、错 pack hash、Worker sandbox 不合格。
   - evidence：evidence/M10/F2S-DEV-M10-001/F2S-WU-M10-001-02/evidence.json、pack-separation.json、forbidden-component-scan.json。
   - dependsOn：[F2S-DEV-M09-001, F2S-WU-M10-001-01]。
   - parallelSafety：shared-lock:m10-001。
   - rollback：从 manifest 物理移除失败 pack；Core 包不重建为含 AI 的单体。
   - estimate：1.5d。
7. **正向验收**：普通用户可安装/修复/卸载 Core；可选 pack 独立；Core 在无 pack、无 Spine、无开发工具时启动。
8. **负向与故障验收**：WebView2 缺失显示诊断；禁止组件、未知许可、pack eligibility=false、项目删除、管理员/网络强依赖均阻断发布。
9. **证据**：evidence/M10/F2S-DEV-M10-001/evidence.json（逻辑 ID F2S-EVD-M10-001）绑定三个包的文件/hash/许可/eligibility。
10. **回滚**：撤销新安装器并恢复上一签名/内部候选；升级/卸载不得触碰用户项目。
11. **完成定义**：2 WU、布局/许可/分包/卸载负测和同号 EVD 完整。
12. **退出状态**：DONE 表示包可生成；未有商业证书时仍是 internal-unsigned-candidate。

## F2S-DEV-M10-002 | 签名原生launcher

1. **任务头**：目标是实现根目录可双击的原生 launcher、主程序身份验证和健康握手；非目标是脚本启动器、自更新器、安装器或通用进程执行器。估算 4.5 人日。
2. **追踪**：F2S-FR-APP-001、F2S-FR-APP-002、F2S-INSTALL-ENTRY-001；测试 F2S-TST-E2E-008、F2S-TST-071；上游 F2S-DEV-M10-001、F2S-DEV-M09-003；下游 F2S-DEV-M10-005、F2S-DEV-M10-006、F2S-DEV-M11-001、F2S-DEV-M11-004；P0。
3. **输入**：已构建 FlashToSpine.exe、expected publisher/signature policy、install layout、health protocol、稳定错误码与简中资源。
4. **输出**：apps/launcher-native/src/main.rs、apps/launcher-native/src/verify.rs、apps/launcher-native/src/health.rs、tests/launcher/launcher.spec.ts；构建产物根目录 FlashToSpineLauncher.exe 与安装目录同名 launcher。
5. **设计边界**：GetModuleFileNameW 定位自身，Win32 参数数组/CreateProcessW 启动；不依赖 cwd；不经 shell；不下载/编译/提权；只验证并启动相邻受信 main；15 秒健康超时后 TaskDialog 显示稳定诊断。
6. **work units**：

##### F2S-WU-M10-002-01 — 原生定位与安全启动

   - output：native launcher 入口、路径/参数/进程启动与单实例交接。
   - reads：install layout、APP-001/002、Windows path policy。
   - writes：apps/launcher-native/src/main.rs。
   - steps：以 GetModuleFileNameW 得自身目录；canonicalize 相邻 main；拒绝 junction/非文件/项目内替换；安全转发允许的项目路径参数；CreateProcessW 不经 shell；记录稳定 exit category。
   - command：npm run build:core。
   - tests：从任意 cwd、中文/空格/长路径双击；shell 元字符；main 缺失/替换；多实例。
   - evidence：evidence/M10/F2S-DEV-M10-002/F2S-WU-M10-002-01/native-launch-tests.json、build-core.log。
   - dependsOn：[F2S-DEV-M10-001]。
   - parallelSafety：shared-lock:m10-002。
   - rollback：恢复上一 launcher 二进制；不修改项目或安装内容。
   - estimate：1.5d。

##### F2S-WU-M10-002-02 — 身份与健康握手

   - output：主程序 hash/signature 验证、随机 named pipe+nonce 健康协议与 TaskDialog 诊断。
   - reads：signature policy、main manifest、diagnostic catalog。
   - writes：apps/launcher-native/src/verify.rs、apps/launcher-native/src/health.rs。
   - steps：验证路径、manifest hash 和适用 Authenticode publisher；创建随机 pipe/nonce；启动 main 并等待版本/nonce/PID handshake 最多 15 秒；超时/错 nonce/进程提前退出显示 TaskDialog，不循环重启。
   - command：npm run test:integration。
   - tests：错签名/hash、重放 nonce、劫持 pipe、迟到握手、主程序崩溃、诊断无 shell。
   - evidence：evidence/M10/F2S-DEV-M10-002/F2S-WU-M10-002-02/health-handshake.json、identity-negative-tests.json。
   - dependsOn：[F2S-WU-M10-002-01, F2S-DEV-M09-003]。
   - parallelSafety：shared-lock:m10-002。
   - rollback：恢复只启动同一已验 manifest main 的上一协议版本；不允许关闭身份校验作为降级。
   - estimate：1.5d。

##### F2S-WU-M10-002-03 — 根入口物化与签名状态

   - output：根入口复制规则、launcher 文件清单与签名/未签名状态测试。
   - reads：F2S-WU-M10-002-01 与 F2S-WU-M10-002-02 binary、用户本地配置中的只读 publisher identity reference 与发行 channel policy；不读取 F2S-DEV-M10-006 输出。
   - writes：tests/launcher/launcher.spec.ts、tools/package-root-entry.ps1。
   - steps：从受信 build staging 复制唯一 pre-sign launcher 到仓库/发行根；记录 hash 并生成 F2S-DEV-M10-006 的签名输入；本阶段明确 internal unsigned，真实 publisher/timestamp 由 F2S-DEV-M10-006 后续裁决；双击 smoke 不触发 build/download。
   - command：npm run release:verify。
   - tests：F2S-TST-E2E-008、F2S-TST-071；根入口存在/可启动/无 shell/无网络/无开发工具依赖。
   - evidence：evidence/M10/F2S-DEV-M10-002/F2S-WU-M10-002-03/evidence.json、root-entry-smoke.json、signature-status.json；缺证书为 NOT_RUN/EXTERNAL。
   - dependsOn：[F2S-WU-M10-002-02]。
   - parallelSafety：shared-lock:m10-002。
   - rollback：根入口回退上一已知 hash；不得以 .cmd 替代。
   - estimate：1.5d。
7. **正向验收**：用户从任意 cwd 双击根 FlashToSpineLauncher.exe，正确 main 经身份校验并在 15 秒内完成 nonce 健康握手。
8. **负向与故障验收**：main 缺失/替换/错签名、pipe 劫持、错 nonce、超时、中文/长路径错误均给稳定诊断；launcher 不下载、不构建、不提权、不经 shell。
9. **证据**：evidence/M10/F2S-DEV-M10-002/evidence.json（逻辑 ID F2S-EVD-M10-002）；商业签名缺失分支明确 NOT_RUN/EXTERNAL。
10. **回滚**：原子替换回上一 launcher；不修改 main、用户项目或更新序列。
11. **完成定义**：3 WU、根入口、身份/握手/异常矩阵、签名外部状态和同号 EVD 完整。
12. **退出状态**：DONE 可为原生 internal unsigned launcher；SignedCommercialReady 仅在 F2S-DEV-M10-006 真实证书链通过后成立。

## F2S-DEV-M10-003 | 更新与文件关联

1. **任务头**：目标是实现签名更新 manifest、反回滚和安全 .f2slink 轻量入口关联；非目标是 launcher 自更新、静默跨通道降级、在链接内嵌资产或 shell 拼接路径。估算 3.0 人日。
2. **追踪**：F2S-FR-APP-004、F2S-AC-APP-001、F2S-REL-UPDATE-001；测试 F2S-TST-E2E-008 与更新安全向量；上游 F2S-DEV-M10-001、F2S-DEV-M10-002、F2S-DEV-M10-006；下游 F2S-DEV-M10-005、F2S-DEV-M11-001、F2S-DEV-M11-004；P1。
3. **输入**：signed update manifest、securityEpoch/sequence/channel、package hash/size、publisher trust、当前安装状态、版本化 .f2slink 与项目路径。
4. **输出**：crates/adapters/update/src/lib.rs、crates/adapters/update/src/manifest.rs、installer/nsis/file-association.nsh、tests/update/update-security.spec.ts。
5. **设计边界**：更新由主程序显式触发且先下载到受限 staging；验签/hash/size/epoch/sequence 后交安装器；launcher 不联网；.f2slink 只含版本化轻量定位信息、不含资产，文件关联使用直接 executable+参数且不经 shell。
6. **work units**：

##### F2S-WU-M10-003-01 — 更新信任与反回滚

   - output：manifest verifier、channel/epoch/sequence 状态机与受限下载提交。
   - reads：F2S-DEV-M10-006 trust root、当前安装 manifest、update policy。
   - writes：crates/adapters/update/src/lib.rs、crates/adapters/update/src/manifest.rs。
   - steps：先验 schema/size；验证签名与 package hash；同 channel sequence 单调、securityEpoch 不回退；拒绝跨 edition/channel；下载失败保留当前版本；用户确认后启动安装器。
   - command：npm run test:integration。
   - tests：错签名/hash/size、重放、回滚、分叉、跨通道、截断下载、断网、当前版本运行中。
   - evidence：evidence/M10/F2S-DEV-M10-003/F2S-WU-M10-003-01/update-security.json、integration.log。
   - dependsOn：[F2S-DEV-M10-001, F2S-DEV-M10-006]。
   - parallelSafety：shared-lock:m10-003。
   - rollback：保留当前完整版本并隔离下载；不降低 epoch/sequence。
   - estimate：1.5d。

##### F2S-WU-M10-003-02 — 文件关联与升级保留

   - output：.f2slink per-user association、轻量链接 schema/open-project 参数合同与 install/upgrade/repair/uninstall 测试。
   - reads：launcher/main CLI contract、NSIS layout、project path policy。
   - writes：installer/nsis/file-association.nsh、tests/update/update-security.spec.ts。
   - steps：注册 per-user ProgID/icon/open command；链接路径作为独立参数；先校验 .f2slink schema 再解析 canonical project path；拒绝内嵌资产与越界链接；支持中文/空格/长路径；升级/修复保持关联；卸载仅移除自身 registration，不删项目。
   - command：npm run release:verify。
   - tests：shell 元字符路径、关联劫持、错 ProgID、损坏/越界/内嵌资产链接、双击 .f2slink、升级/卸载项目保留。
   - evidence：evidence/M10/F2S-DEV-M10-003/F2S-WU-M10-003-02/evidence.json、file-association-matrix.json。
   - dependsOn：[F2S-DEV-M10-002, F2S-WU-M10-003-01]。
   - parallelSafety：sequential。
   - rollback：恢复上一 ProgID/安装版本；不修改项目内容。
   - estimate：1.5d。
7. **正向验收**：新 sequence/epoch 的受信包可经用户确认升级；双击版本化 .f2slink 安全打开对应项目且链接不含资产，升级/修复后仍有效。
8. **负向与故障验收**：重放、降级、错签名、跨通道、下载截断、关联参数注入、卸载删项目均失败且当前版本可用。
9. **证据**：evidence/M10/F2S-DEV-M10-003/evidence.json（逻辑 ID F2S-EVD-M10-003）绑定 update/file-association 矩阵；真实公网更新源非必需且不得默认配置。
10. **回滚**：回到上一已安装版本与 association；securityEpoch/sequence 不回滚。
11. **完成定义**：2 WU、更新信任/反回滚/关联/项目保留与同号 EVD 完整。
12. **退出状态**：DONE 表示更新协议与本地矩阵完成；无真实签名源时公开更新为 NOT_RUN/EXTERNAL。

## F2S-DEV-M10-004 | 路径/DPI/a11y/i18n

1. **任务头**：目标是完成 Windows 路径、DPI、键盘/焦点、非颜色状态、对比度与简中资源发布验收；非目标是仅靠截图人工判断或为修复 DPI 降低源资产分辨率。估算 3.0 人日。
2. **追踪**：F2S-NFR-A11Y-001、F2S-NFR-A11Y-002、F2S-NFR-A11Y-003、F2S-NFR-I18N-001、F2S-NFR-DPI-001；测试对应 F2S-TST-E2E-A11Y-001、F2S-TST-E2E-A11Y-002、F2S-TST-E2E-A11Y-003、F2S-TST-E2E-I18N-001、F2S-TST-E2E-DPI-001、F2S-TST-080、F2S-TST-088；上游 F2S-DEV-M01-003、F2S-DEV-M07-006、F2S-DEV-M09-005；下游 F2S-DEV-M10-005、F2S-DEV-M11-001；P0。
3. **输入**：UI route/state inventory、design tokens、简中 catalog、Windows path fixtures、100/125/150/200% DPI 矩阵。
4. **输出**：apps/desktop-ui/src/platform/windows-a11y.ts、apps/desktop-ui/src/features/release/i18n/zh-CN.json、tests/ui/windows-accessibility.spec.ts、tests/ui/windows-path-dpi.spec.ts；M01-003 拥有的全局 apps/desktop-ui/src/i18n/zh-CN.json 只读。
5. **设计边界**：所有关键操作可键盘完成；焦点可见且恢复；状态不只靠颜色；正文 token ≥4.5:1、控件边界 ≥3:1；逻辑/物理坐标转换集中；文件路径由 Rust 验证。
6. **work units**：

##### F2S-WU-M10-004-01 — 键盘焦点与简中

   - output：Windows a11y adapter、简中资源与 UI 自动检查。
   - reads：route/action inventory、token catalog、错误码。
   - writes：apps/desktop-ui/src/platform/windows-a11y.ts、apps/desktop-ui/src/features/release/i18n/zh-CN.json、tests/ui/windows-accessibility.spec.ts。
   - steps：补语义名称/role/state；定义 Tab/方向键/快捷键与 modal focus trap/restore；错误/审批/命中状态加文本/图形；检查硬编码文本与资源缺键。
   - command：npm run test:integration。
   - tests：F2S-TST-E2E-A11Y-001、F2S-TST-E2E-A11Y-002、F2S-TST-E2E-A11Y-003、F2S-TST-E2E-I18N-001、F2S-TST-088。
   - evidence：evidence/M10/F2S-DEV-M10-004/F2S-WU-M10-004-01/a11y-i18n-report.json、keyboard-flows.json。
   - dependsOn：[F2S-DEV-M07-006]。
   - parallelSafety：shared-lock:m10-004。
   - rollback：恢复上一资源/交互实现；不可移除键盘路径作为降级。
   - estimate：1.5d。

##### F2S-WU-M10-004-02 — 路径DPI与对比矩阵

   - output：中文/空格/长路径与 4 档 DPI 自动矩阵、对比度报告。
   - reads：F2S-DEV-M09-005 renderer traces、design tokens、path fixtures。
   - writes：tests/ui/windows-path-dpi.spec.ts。
   - steps：每 DPI 验 hit-test、骨骼拖动、时间轴、截图尺寸与 context restore；每路径跑 create/open/save/export/publish；计算 token/控件对比；禁止坐标漂移和模糊掩盖。
   - command：npm run release:verify。
   - tests：F2S-TST-E2E-DPI-001、F2S-TST-080、F2S-TST-088；100/125/150/200% 逐格结果。
   - evidence：evidence/M10/F2S-DEV-M10-004/F2S-WU-M10-004-02/evidence.json、path-dpi-matrix.json、contrast-report.json。
   - dependsOn：[F2S-DEV-M09-005, F2S-WU-M10-004-01]。
   - parallelSafety：sequential。
   - rollback：回退导致 DPI/path 退化的 UI 变更；源资产不降质。
   - estimate：1.5d。
7. **正向验收**：关键流程键盘可达、焦点可见、简中无缺键；四 DPI 和三类复杂路径全链通过；对比阈值满足。
8. **负向与故障验收**：焦点丢失、颜色唯一表达、硬编码英文、坐标偏移、路径截断、对比不足或源资产降质均阻断。
9. **证据**：evidence/M10/F2S-DEV-M10-004/evidence.json（逻辑 ID F2S-EVD-M10-004）含逐格截图 hash、自动断言与环境。
10. **回滚**：恢复上一 token/layout/path adapter；不降低阈值或删除失败场景。
11. **完成定义**：2 WU、全部 exact A11Y/I18N/DPI/path 测试与同号 EVD 完整。
12. **退出状态**：DONE 只覆盖声明矩阵；未测显示器/缩放组合不外推。

## F2S-DEV-M10-005 | 干净机与Windows矩阵

1. **任务头**：目标是在干净 Windows 11 P0 与 Windows 10 22H2 P1 环境验证安装、入口、核心链、更新、恢复和卸载；非目标是以开发机结果或手工口述代替 clean VM evidence。估算 4.5 人日。
2. **追踪**：F2S-FR-APP-003、F2S-NFR-COMPAT-001、F2S-NFR-COMPAT-002；测试 F2S-TST-E2E-COMPAT-001、F2S-TST-E2E-COMPAT-002、F2S-TST-070、F2S-TST-079、F2S-TST-119；上游 F2S-DEV-M10-001、F2S-DEV-M10-002、F2S-DEV-M10-003、F2S-DEV-M10-004、F2S-DEV-M09-005、F2S-DEV-M09-006；下游 F2S-DEV-M10-006、F2S-DEV-M11-001、F2S-DEV-M11-004；P0/P1。
3. **输入**：冻结安装包/hash、clean VM images、系统 Evergreen WebView2、普通用户账户、路径/DPI fixtures、无 Spine/Python/CUDA/model 基线。
4. **输出**：tools/windows-matrix/run-clean-vm.ps1、tests/windows-matrix/scenarios.json、tests/windows-matrix/assertions.spec.ts；执行期 win11-p0.json、win10-p1.json 与截图/日志。
5. **设计边界**：VM 来源/build/hash 可审计；安装前证明无开发工具与产品残留；Win10 失败如实记录但不冒充 P0，也不自动阻断已通过 Win11 的功能，除非共享 P0 defect。
6. **work units**：

##### F2S-WU-M10-005-01 — Clean VM矩阵Harness

   - output：可重置 VM 场景定义、前置/后置扫描和证据采集器。
   - reads：installer/launcher/update contracts、environment registry。
   - writes：tools/windows-matrix/run-clean-vm.ps1、tests/windows-matrix/scenarios.json。
   - steps：验证 OS build/VM hash/普通用户/无产品残留；安装冻结 hash；运行场景；采集 event/log/screenshot/package inventory；恢复 snapshot；脚本不得保存真实秘密。
   - command：npm run test:integration。
   - tests：Harness 对错 VM/hash、非 clean、管理员账户、缺 WebView2 的拒绝。
   - evidence：evidence/M10/F2S-DEV-M10-005/F2S-WU-M10-005-01/harness-attestation.json。
   - dependsOn：[F2S-DEV-M10-001, F2S-DEV-M10-002, F2S-DEV-M10-003, F2S-DEV-M10-004]。
   - parallelSafety：sequential。
   - rollback：丢弃测试 VM snapshot；不操作宿主用户项目。
   - estimate：1.5d。

##### F2S-WU-M10-005-02 — Windows11 P0矩阵

   - output：Win11 clean install→root launch→核心全链→repair/update/recovery→uninstall 结果。
   - reads：F2S-WU-M10-005-01、冻结包、F2S-DEV-M09-005 quality report、F2S-DEV-M09-006 fault report。
   - writes：tests/windows-matrix/assertions.spec.ts、执行期 win11-p0.json。
   - steps：普通用户、中文/空格/长路径、四 DPI；无 AI/Spine 时创建/导入图片/审批/编辑/Rig IR预览/开放导出；安装可选 eligible pack 条件分支；kill/recovery；项目保留。
   - command：npm run release:verify。
   - tests：F2S-TST-E2E-COMPAT-001、F2S-TST-070、F2S-TST-079、F2S-TST-119、F2S-TST-E2E-008。
   - evidence：evidence/M10/F2S-DEV-M10-005/F2S-WU-M10-005-02/win11-p0.json、win11-artifacts-index.json。
   - dependsOn：[F2S-WU-M10-005-01, F2S-DEV-M09-005, F2S-DEV-M09-006]。
   - parallelSafety：sequential。
   - rollback：拒绝候选包并保留上一版；VM 回快照。
   - estimate：1.5d。

##### F2S-WU-M10-005-03 — Windows10 P1矩阵

   - output：Win10 22H2 尽力兼容矩阵、限制与 shared-defect 分类。
   - reads：同一冻结包/场景、Win10 clean VM。
   - writes：执行期 win10-p1.json、known-limitations.json。
   - steps：复用 P0 场景并记录 pass/fail/not-run；区分平台限制与共享缺陷；不得把 Win11 结果复制为 Win10；限制进入用户文档。
   - command：npm run release:verify。
   - tests：F2S-TST-E2E-COMPAT-002、F2S-TST-070、F2S-TST-079、F2S-TST-119。
   - evidence：evidence/M10/F2S-DEV-M10-005/F2S-WU-M10-005-03/evidence.json、win10-p1.json、known-limitations.json。
   - dependsOn：[F2S-WU-M10-005-01]。
   - parallelSafety：sequential。
   - rollback：不发布错误 Win10 声明；不回退 Win11 已验证能力，除非发现共享 P0 缺陷。
   - estimate：1.5d。
7. **正向验收**：Win11 P0 全矩阵通过；Win10 每格有真实状态和限制；两者绑定同一候选 hash。
8. **负向与故障验收**：开发机/非 clean、管理员、包 hash 漂移、复制结果、缺证据、卸载删项目、无 AI/Spine 时 Core 失效均阻断。
9. **证据**：evidence/M10/F2S-DEV-M10-005/evidence.json（逻辑 ID F2S-EVD-M10-005）索引两 OS；不可用 VM 项为 NOT_RUN/EXTERNAL。
10. **回滚**：候选回退上一已验证包；测试 VM 回快照；用户项目不回滚。
11. **完成定义**：3 WU、Win11 P0 与 Win10 P1、复杂路径/DPI/恢复/卸载和同号 EVD 完整。
12. **退出状态**：DONE 要求 Win11 P0 实证；Win10 可 DONE_WITH_LIMITATIONS，但不得伪称全支持。

## F2S-DEV-M10-006 | 签名与发布身份配置

1. **任务头**：目标是建立应用 manifest、Authenticode 签名/验证、publisher identity 与 timestamp 门；非目标是生成自签证书冒充商业主体或在证书缺失时声称已签名发布。估算 3.0 人日。
2. **追踪**：F2S-NFR-LIC-002、F2S-FR-APP-003、F2S-REL-SIGN-001；测试 F2S-TST-E2E-LIC-002、F2S-TST-E2E-008；上游 F2S-DEV-M09-004、F2S-DEV-M10-001、F2S-WU-M10-002-02、F2S-WU-M10-002-03；下游 F2S-DEV-M10-003、F2S-DEV-M10-005、F2S-DEV-M11-004；P0。
3. **输入**：package/launcher/main hashes、SBOM/notices、publisher policy、代码签名证书/HSM 引用与 timestamp service（若存在）。
4. **输出**：tools/signing/build-application-manifest.ps1、tools/signing/verify-signatures.ps1、tests/signing/signature-policy.spec.ts；执行期 application-manifest.json/.sig 与签名报告。
5. **设计边界**：私钥不入仓库/日志/项目；签名前 manifest 非自引用，排除自身与 .sig；checksums 排除自身/manifest/sig；签名后禁止改字节；synthetic cert 只做合同测试。
6. **work units**：

##### F2S-WU-M10-006-01 — 发布Manifest与签名合同

   - output：非自引用 manifest/checksums 生成器与签名策略测试。
   - reads：F2S-DEV-M10-001 package layouts、F2S-DEV-M09-004 inventories、version/channel/securityEpoch。
   - writes：tools/signing/build-application-manifest.ps1、tests/signing/signature-policy.spec.ts。
   - steps：稳定排序 artifact path/hash/size/pack/SBOM；排除 manifest/.sig/checksums 自身；绑定 publisher/channel/version/epoch；用 synthetic key 测签/验、错算法/错 hash/篡改。
   - command：npm run release:verify。
   - tests：非自引用、双 hash 一致、artifact 增删改、算法 allowlist、私钥 canary=0。
   - evidence：evidence/M10/F2S-DEV-M10-006/F2S-WU-M10-006-01/manifest-contract.json、synthetic-signature-tests.json。
   - dependsOn：[F2S-DEV-M09-004, F2S-DEV-M10-001, F2S-WU-M10-002-02]。
   - parallelSafety：shared-lock:m10-006。
   - rollback：恢复上一 manifest generator；新 artifact 保持不可发布。
   - estimate：1.5d。

##### F2S-WU-M10-006-02 — 真实证书条件签名与验签

   - output：launcher/main/installer/update artifacts 的签名、timestamp 与 publisher 验证报告。
   - reads：F2S-WU-M10-006-01 manifest、受控证书/HSM 与 timestamp（若存在）。
   - writes：tools/signing/verify-signatures.ps1、执行期 application-manifest.json/.sig。
   - steps：签名前冻结 hash；调用受控签名设施；逐文件 WinVerifyTrust/publisher/timestamp；签名后重算 manifest contract；缺证书时生成 internal-unsigned 状态并阻断 commercial/public-update。
   - command：npm run release:verify。
   - tests：错 publisher、过期/撤销/无 timestamp、签后篡改、部分文件未签、证书缺失。
   - evidence：evidence/M10/F2S-DEV-M10-006/F2S-WU-M10-006-02/evidence.json、signature-report.json；证书/正式 enrollment 缺失为 NOT_RUN/EXTERNAL。
   - dependsOn：[F2S-WU-M10-006-01, F2S-WU-M10-002-03]。
   - parallelSafety：sequential。
   - rollback：废弃候选并回上一签名版本；绝不回滚 securityEpoch。
   - estimate：1.5d。
7. **正向验收**：若真实证书存在，所有可执行/安装/更新 artifact 的 publisher/timestamp/hash 一致；无证书时内部候选状态清晰。
8. **负向与故障验收**：自签 fixture、错 publisher、缺签、签后篡改、manifest 自引用、私钥泄漏、证书缺失却 commercialReady=true 均失败。
9. **证据**：evidence/M10/F2S-DEV-M10-006/evidence.json（逻辑 ID F2S-EVD-M10-006）；真实商业证书缺失必须 NOT_RUN/EXTERNAL。
10. **回滚**：回到上一已签名候选或无公开发布；不复用撤销/过期证书。
11. **完成定义**：2 WU、manifest、synthetic 合同、真实条件验签、外部状态与同号 EVD 完整。
12. **退出状态**：DONE 可表示签名基础设施完成；CommercialSignedReady 只在真实证书证据通过后为 true。

## 里程碑退出门

1. 6 个 DEV、14 个唯一 WU、6 个同号 EVD 出口存在，全部证据位于 evidence/M10/F2S-DEV-M10-xxx/。
2. 根目录 FlashToSpineLauncher.exe 是唯一双击入口；不得存在以脚本替代的发布路径。
3. Core 禁止组件扫描为 0；Worker/Model Pack 独立且受 F2S-DEV-M09-001 eligibility 与 F2S-DEV-M09-004 许可门控制。
4. Win11 clean VM P0 必须实证；Win10 P1 缺失或限制如实记录。
5. code-signing cert 缺失时 commercial signing=NOT_RUN/EXTERNAL；内部未签名候选不能被 F2S-DOC-DEVPLAN-M11-001 改写为公开发布 PASS。
6. F2S-DOC-DEVPLAN-M11-001 只消费冻结候选 hash、入口/矩阵/签名证据及明确外部状态。
