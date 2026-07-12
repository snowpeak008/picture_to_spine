---
doc_id: F2S-NEWPIC-DEVPLAN-001
revision: 0.1
status: draft
created_at: 2026-07-12
source_design:
  - plan/NEWpic_reload/01-PSD分层资源包输入重设计.md
canonical_for:
  - F2S-NEWPIC-DEV-SEQUENCE-001
  - F2S-NEWPIC-DEV-LAYERPACK-001
  - F2S-NEWPIC-DEV-STUDIO-001
depends_on:
  - F2S-NEWPIC-RELOAD-001
---

# PSD / 分层资源包输入重设计原子开发计划

## 1. 执行目标

本计划把 `plan/NEWpic_reload/01-PSD分层资源包输入重设计.md` 转成可按序号执行的原子开发任务。核心路线是：

1. 先实现 **透明 PNG 文件夹 LayerPack 导入**，绕开当前手绘遮罩痛点。
2. 再实现 **允许重叠的新 QA 和审批链路**，让 LayerPack 可以成为 Layer Gate 主路径。
3. 再实现 **粗 Rig / 标准 Rig 生成**，把 LayerPack 接到后续动画流程。
4. 再扩展 **PSD 普通层读取**。
5. 最后实现 **LayerPack Studio MVP**，服务没有 PS/Krita/GIMP/Clip Studio 的用户。

本计划不把公网服务、Photoshop 便携版、模型权重或第三方图像工具捆绑进核心包。所有外部/AI 能力只作为可选候选，不能自动审批。

## 2. 执行规则

- 任务必须按 `NRD-001` 到 `NRD-031` 顺序执行；除非某任务明确标注 `parallelSafe`。
- 每个任务完成后必须能独立 commit。
- 每个任务只能写自己列出的路径；跨层合同先改 schema/DTO，再改 Domain/Application/Adapter/UI。
- 任何涉及用户图片、LayerPack、PSD、透明 PNG 的路径必须通过 Native/Rust 侧预检；WebView 不直接解码原始文件。
- 现有手绘遮罩功能保留，但新入口优先级高于手绘入口。
- 所有执行证据只记录 hash、文件名、尺寸、状态和脱敏路径 token，不记录用户本地绝对路径和原始角色图。

## 3. 最小可用切片

第一阶段 MVP 到 `NRD-019` 为止：

```text
LayerPack schema
-> 自制 fixture
-> Domain/Application 预检
-> Native 文件夹导入
-> PNG Alpha 入 CAS
-> UI 导入向导
-> 合成预览和 QA
-> 创建 LayerSet 候选
-> 人工批准 Layer Gate
```

完成 `NRD-019` 后，用户应能不用手绘遮罩，直接通过分层透明 PNG 资源包完成分层审批。

## 4. 路径所有权

| 路径 | 任务段 | 说明 |
| --- | --- | --- |
| `schemas/src/layerpack.schema.json` | NRD-002 | LayerPack 合同源文件 |
| `schemas/generated/**` | NRD-003 | 生成类型和 golden，禁止手改生成物 |
| `fixtures/layerpacks/**` | NRD-004 | 自制几何测试资源包 |
| `crates/domain/src/layerpack/**` | NRD-005..007 | Domain 实体、策略、QA |
| `crates/application/src/layerpack/**` | NRD-008..009 | use case 和 ports |
| `crates/adapters/src/layerpack/**` | NRD-011..012, NRD-025 | 文件夹、PNG、PSD adapter |
| `apps/desktop/src-tauri/src/ipc_host.rs` | NRD-010, NRD-013, NRD-019 | IPC composition 和 Native 对话框 |
| `apps/desktop-ui/src/features/layerpack/**` | NRD-014..018, NRD-026..028 | 导入向导和 Studio |
| `apps/desktop-ui/src/features/layers/**` | NRD-020 | 手绘入口降级和跳转 |
| `crates/application/src/rig/**` / `crates/domain/src/rig/**` | NRD-021..022 | LayerPack -> Rig 候选 |
| `docs/user/**` | NRD-029 | 用户说明 |
| `tests/**` | 每任务 | 对应单元/集成/E2E |

## 5. 原子任务总览

| 序号 | 任务 | 主要产出 | 退出条件 |
| ---: | --- | --- | --- |
| NRD-001 | 冻结 LayerPack 术语和 feature flag | 术语/开关文档 | 新流程可关闭 |
| NRD-002 | LayerPack JSON Schema | schema 源文件 | 正反例可校验 |
| NRD-003 | 生成类型和 golden | Rust/TS 类型 | schema 生成稳定 |
| NRD-004 | 自制 LayerPack fixture | valid/invalid 资源包 | 无版权素材 |
| NRD-005 | Domain 实体 | LayerPackManifest 等 | 不依赖文件系统 |
| NRD-006 | Domain policy | role/drawOrder/socket 校验 | 负例全拒绝 |
| NRD-007 | 合成 QA 领域模型 | QaReport | 允许重叠但可解释 |
| NRD-008 | Application preflight port | 预检 use case | 只返回投影 |
| NRD-009 | LayerPack -> LayerSet | 候选创建 use case | 不自动批准 |
| NRD-010 | Native 文件夹选择 IPC | folder picker | 不泄漏绝对路径 |
| NRD-011 | 文件系统 adapter | manifest/path safety | 防路径越界 |
| NRD-012 | PNG/Alpha adapter | PNG 入 CAS | Alpha/尺寸可验证 |
| NRD-013 | IPC DTO 接线 | 前后端合同 | stale revision 拒绝 |
| NRD-014 | UI 入口和 store | LayerPack workspace | 入口优先于手绘 |
| NRD-015 | UI 结构预检页 | 文件树/错误表 | 阻断信息可读 |
| NRD-016 | UI 映射和层级编辑 | role/zIndex editor | 用户可修 manifest |
| NRD-017 | UI pivot/socket 编辑 | pivot/socket editor | weapon-grip 可确认 |
| NRD-018 | UI 合成预览 | side 对照 | 偏移/漏层可见 |
| NRD-019 | Layer Gate 审批接入 | 审批闭环 | 不手绘可批准 |
| NRD-020 | 手绘工作台降级 | repair mode | 旧功能仍可用 |
| NRD-021 | Coarse Rig 生成 | 粗 Rig 候选 | 标记 COARSE_RIG |
| NRD-022 | Standard Rig 生成 | 17 层 Rig 候选 | 绑定 pivot/boneHint |
| NRD-023 | 导出 provenance 更新 | manifest 标记 | 导出可追踪来源 |
| NRD-024 | PSD 解析技术 spike | 决策报告 | 不先引入依赖 |
| NRD-025 | PSD 普通层 adapter | PSD -> LayerPackDraft | 失败可降级 PNG |
| NRD-026 | LayerPack Studio 数据模型 | Studio project draft | 可保存草稿 |
| NRD-027 | Studio 拖拽摆放画布 | transform editor | 可移动缩放旋转 |
| NRD-028 | Studio 导出资源包 | LayerPack export | 无 PS 可产出包 |
| NRD-029 | 用户文档和模板 | quick start | 用户知道准备什么 |
| NRD-030 | 端到端回归 | MVP E2E | 从包到 Layer Gate |
| NRD-031 | 本地 AI 候选 spike | NOT_RUN/EXTERNAL 报告 | 默认不启用 |

## 6. 详细任务卡

### NRD-001 — 冻结 LayerPack 术语和 feature flag

- 目标：把 LayerPack 新入口作为可开关能力引入，不影响现有导入/手绘链。
- 写入：
  - `plan/NEWpic_reload/dev/00-PSD分层资源包原子开发计划.md`
  - `docs/limits/known-limitations.md`
  - `PROJECT_MEMORY.md`
- 步骤：
  1. 补充术语：LayerPack、reference-view、layer-source、master-composite、coarse rig、standard rig。
  2. 记录 feature flag：`F2S_LAYERPACK_IMPORT=off|on`，默认 off，开发构建可开。
  3. 明确当前手绘遮罩保留为 repair mode。
  4. 记录“不捆绑 Photoshop / 公网服务 / 模型权重”的边界。
- 验收：
  - 文档明确新入口默认不改变现有用户流程。
  - 产品边界文档没有把 AI/PSD 能力写成已验证。
- 回滚：删除 feature flag 说明和术语补充，不改源码。
- 依赖：无。
- 估算：0.25d。

### NRD-002 — 新增 LayerPack JSON Schema

- 目标：定义 `manifest.f2s-layerpack.json` 的可测试合同。
- 写入：
  - `schemas/src/layerpack.schema.json`
  - `schemas/src/common.schema.json` 仅在需要复用 `sha256` / `relativePath` 时提案式修改
  - `tests/unit/layerpack-schema.test.mjs`
- 步骤：
  1. 建立 `schemaVersion = "f2s.layerpack/1"`。
  2. 定义 `packId`、`characterSlug`、`rigProfile`、`canvas`、`views`、`sources`、`layers`、`drawOrder`、`sockets`、`qaPolicy`、`provenance`。
  3. 限制相对路径不得为空、不得包含盘符、不得以 `/`、`\`、`..` 越界。
  4. 限制 `rigProfile` 为 `standard-side-humanoid-v1 | coarse-side-humanoid-v1 | single-piece-reference`。
  5. 增加 schema 正反例测试。
- 验收：
  - 合法 standard/coarse manifest 通过。
  - 缺 side view、缺 weapon socket、路径越界、重复 role、无 Alpha 声明均失败。
- 回滚：删除 schema 和单测。
- 依赖：NRD-001。
- 估算：1d。

### NRD-003 — 生成 Rust/TypeScript 类型和 golden

- 目标：把 LayerPack schema 接入现有 schema 生成链。
- 写入：
  - `schemas/generate.mjs`
  - `schemas/generated/rust/**`
  - `schemas/generated/ts/**`
  - `schemas/generated/golden/layerpack-*.json`
- 步骤：
  1. 将 `layerpack.schema.json` 纳入生成入口。
  2. 新增 canonical golden 和 adversarial golden。
  3. 保证生成结果稳定排序。
  4. 更新 schema storage/unit 测试。
- 验收：
  - `npm test -- tests/unit/storage-schema.test.mjs` 通过。
  - 连续运行 schema 生成无 diff。
- 回滚：撤回生成入口和生成物。
- 依赖：NRD-002。
- 估算：0.5d。

### NRD-004 — 创建自制 LayerPack fixture

- 目标：提供无版权、可复现的 LayerPack 测试资源。
- 写入：
  - `fixtures/layerpacks/valid-standard-pack/**`
  - `fixtures/layerpacks/valid-coarse-pack/**`
  - `fixtures/layerpacks/invalid-path-traversal/**`
  - `fixtures/layerpacks/invalid-missing-alpha/**`
  - `fixtures/layerpacks/invalid-empty-layer/**`
  - `tools/fixtures/generate-layerpack-fixtures.mjs`
- 步骤：
  1. 用脚本生成几何透明 PNG，不使用外部角色图。
  2. standard fixture 覆盖 17 role。
  3. coarse fixture 覆盖 7 role。
  4. invalid fixture 覆盖路径越界、缺 Alpha、空层、缺 socket。
  5. 输出 `hashes.sha256` 和 `LICENSES.json`。
- 验收：
  - fixture 总大小小于 5MB。
  - 所有图片来源为程序生成。
  - `hashes.sha256` 可重算一致。
- 回滚：删除 fixture 和生成脚本。
- 依赖：NRD-002。
- 估算：1d。

### NRD-005 — 新增 Domain LayerPack 实体

- 目标：在 Domain 层建立与文件系统无关的 LayerPack 领域模型。
- 写入：
  - `crates/domain/src/layerpack/mod.rs`
  - `crates/domain/src/layerpack/model.rs`
  - `crates/domain/src/lib.rs`
  - `crates/domain/tests/layerpack_model.rs`
- 步骤：
  1. 定义 `LayerPackManifest`、`LayerPackLayer`、`ReferenceView`、`LayerPackSocket`、`LayerPackCanvas`。
  2. 定义 role/profile 枚举。
  3. 定义相对路径值对象，只保存相对路径。
  4. 定义 `LayerPackDraft` 与 `LayerPackApprovedInput` 分离。
- 验收：
  - Domain 不引用 `std::fs`、Tauri、React、PNG decoder。
  - 非法 slug、空 canvas、重复 layerId、重复 zIndex 被拒绝。
- 回滚：删除 `layerpack` 模块和测试。
- 依赖：NRD-003。
- 估算：1d。

### NRD-006 — 实现 LayerPackPolicy

- 目标：把 role 完整性、draw order、socket、profile 规则集中成可测试策略。
- 写入：
  - `crates/domain/src/layerpack/policy.rs`
  - `crates/domain/tests/layerpack_policy.rs`
- 步骤：
  1. standard profile 要求 17 role。
  2. coarse profile 要求 7 role。
  3. single-piece-reference 不允许进入正式 Layer Gate。
  4. 主武器必须有 semantic=`weapon-grip` 的 socket。
  5. 每个 required layer 必须可由后续 adapter 证明非空。
- 验收：
  - 正例 standard/coarse 通过。
  - 缺 weapon、重复 zIndex、未知 role、single-piece 正式审批全部失败。
- 回滚：删除 policy 模块和调用点。
- 依赖：NRD-005。
- 估算：0.75d。

### NRD-007 — 定义允许重叠的合成 QA 模型

- 目标：替代“所有像素只能属于一个层”的旧硬约束。
- 写入：
  - `crates/domain/src/layerpack/qa.rs`
  - `crates/domain/tests/layerpack_qa.rs`
- 步骤：
  1. 定义 `LayerPackQaReport`、`QaIssue`、`QaSeverity`。
  2. 区分 `BLOCKED`、`REVIEW`、`INFO`。
  3. 定义 alpha 覆盖差异、RGB 差异、边缘 halo、bbox mismatch 指标。
  4. 明确重叠默认 `REVIEW`，不是 `BLOCKED`。
- 验收：
  - 空 required layer 是 BLOCKED。
  - 大面积未知重叠是 REVIEW。
  - 合成差异超阈值可按 policy BLOCKED。
- 回滚：删除 QA 模型；保持旧 QA 不变。
- 依赖：NRD-006。
- 估算：0.75d。

### NRD-008 — Application 预检 use case 和 ports

- 目标：定义 Native/Adapter 可以实现的 LayerPack 预检入口。
- 写入：
  - `crates/application/src/layerpack/mod.rs`
  - `crates/application/src/layerpack/preflight.rs`
  - `crates/application/src/ports/layerpack_fs.rs`
  - `crates/application/src/ports/image_probe.rs`
  - `crates/application/tests/layerpack_preflight.rs`
- 步骤：
  1. 定义 `preflight_layer_pack(folderToken)`。
  2. Port 只返回 manifest bytes、相对文件列表、图像摘要、hash。
  3. use case 调 Domain policy。
  4. 输出脱敏 projection，不含绝对路径。
- 验收：
  - valid fixture 得到 `QA_PASSED`。
  - invalid fixture 得到结构化 issue。
  - 任何路径越界都在 Application 层前 fail closed。
- 回滚：删除 layerpack application 模块。
- 依赖：NRD-007。
- 估算：1d。

### NRD-009 — Application 创建 LayerSet 候选

- 目标：从已预检 LayerPack 创建 LayerSet candidate，但不批准。
- 写入：
  - `crates/application/src/layerpack/create_layer_set.rs`
  - `crates/application/tests/layerpack_to_layerset.rs`
  - `crates/domain/src/layers/model.rs` 仅做必要扩展
- 步骤：
  1. 将 LayerPack layers 映射成现有 `Layer` / `LayerSet`。
  2. 保存 overlap policy 和 LayerPack source hash。
  3. 不复用旧 recomposition “无重叠” approval gate。
  4. 标记来源为 `LayerPack`.
- 验收：
  - 创建后 `approvalState=PENDING`。
  - 修改 LayerPack 输入 hash 后旧审批失效。
  - single-piece-reference 不能创建正式可导出 LayerSet。
- 回滚：撤回 use case；不迁移旧项目。
- 依赖：NRD-008。
- 估算：1d。

### NRD-010 — Native 文件夹选择 IPC

- 目标：新增受限文件夹选择能力。
- 写入：
  - `apps/desktop/src-tauri/src/ipc_host.rs`
  - `crates/application/src/ports/ipc.rs`
  - `apps/desktop-ui/src/native/ipc.ts`
- 步骤：
  1. 新增 `layerpack.chooseFolder` IPC 方法。
  2. 使用 Windows 原生文件夹选择器。
  3. Native 保存 folder token，不向 WebView 返回绝对路径。
  4. 用户取消返回 `{cancelled:true}`。
- 验收：
  - WebView 只拿 token、folderName、diagnostics。
  - token 会话内有效，重启后失效。
  - 取消不改变项目 revision。
- 回滚：移除 IPC 方法和前端类型。
- 依赖：NRD-008。
- 估算：0.75d。

### NRD-011 — 文件系统 adapter 和路径安全

- 目标：读取 manifest 和相对文件，不允许路径逃逸。
- 写入：
  - `crates/adapters/src/layerpack/fs_reader.rs`
  - `crates/adapters/src/layerpack/mod.rs`
  - `crates/adapters/tests/layerpack_fs.rs`
- 步骤：
  1. 解析 folder token 到 Native 内部路径。
  2. 读取 `manifest.f2s-layerpack.json`。
  3. 规范化相对路径并校验仍位于资源包根。
  4. 拒绝绝对路径、`..`、符号链接越界和私有数据根污染。
- 验收：
  - path traversal fixture 被拒绝。
  - 正常 fixture 文件列表稳定。
  - 不向 UI 返回绝对路径。
- 回滚：删除 fs adapter。
- 依赖：NRD-010。
- 估算：1d。

### NRD-012 — PNG / Alpha adapter 入 CAS

- 目标：读取 LayerPack PNG 层，验证 Alpha，并存入 CAS。
- 写入：
  - `crates/adapters/src/layerpack/png_layers.rs`
  - `crates/adapters/tests/layerpack_png.rs`
  - `apps/desktop/src-tauri/src/ipc_host.rs` 仅接线
- 步骤：
  1. 校验 PNG 格式和 alpha channel。
  2. 支持全画布 PNG。
  3. 为局部裁切 PNG 预留 `offsetPx` 校验。
  4. 计算 SHA-256 并写 CAS。
  5. 生成受限预览图。
- 验收：
  - 缺 Alpha / 空层 / 尺寸不匹配被拒绝。
  - CAS 中只保存 hash 可索引 bytes。
  - UI 只拿缩略预览 data URL。
- 回滚：删除 PNG adapter；CAS 不自动删除已写测试数据。
- 依赖：NRD-011。
- 估算：1d。

### NRD-013 — IPC DTO 合同接线

- 目标：将 `preflight/create/status/approve` 的 DTO 接到前后端。
- 写入：
  - `schemas/src/ipc.schema.json`
  - `schemas/generated/**`
  - `apps/desktop/src-tauri/src/ipc_host.rs`
  - `apps/desktop-ui/src/native/ipc.ts`
  - `tests/unit/layerpack-ipc-contract.test.mjs`
- 步骤：
  1. 定义 `layerpack.preflight`。
  2. 定义 `layerpack.createLayerSet`。
  3. 定义 `layerpack.status`。
  4. 统一 stale revision 错误。
- 验收：
  - DTO schema 校验通过。
  - stale project revision 拒绝且不部分写入。
  - IPC 错误码可被 UI 本地化。
- 回滚：撤回 IPC schema 和生成物。
- 依赖：NRD-012。
- 估算：1d。

### NRD-014 — UI 新增 LayerPack Workspace 入口

- 目标：让用户能从“分层与素材”优先进入资源包导入，而不是手绘。
- 写入：
  - `apps/desktop-ui/src/features/layerpack/LayerPackWorkspace.tsx`
  - `apps/desktop-ui/src/features/layerpack/layerpack.css`
  - `apps/desktop-ui/src/state/projectStore.ts`
  - `apps/desktop-ui/src/app/AppShell.tsx`
- 步骤：
  1. 新建 LayerPack workspace 壳。
  2. 在 Layer gate 未批准时展示“导入分层资源包”主 CTA。
  3. 手绘入口改成次级 CTA。
  4. 加入 busy/error/cancel 状态。
- 验收：
  - 母版未批准时仍锁定。
  - 母版批准后优先看到 LayerPack 导入。
  - 取消文件夹选择不改变状态。
- 回滚：从 AppShell 移除 workspace，恢复旧 LayerWorkspace。
- 依赖：NRD-013。
- 估算：1d。

### NRD-015 — UI 结构预检页

- 目标：显示资源包文件结构和阻断问题。
- 写入：
  - `apps/desktop-ui/src/features/layerpack/LayerPackPreflightPanel.tsx`
  - `apps/desktop-ui/src/features/layerpack/LayerPackIssueList.tsx`
  - `tests/ui/layerpack-preflight.test.mjs`
- 步骤：
  1. 显示 packId、rigProfile、canvas、side view、layer count。
  2. 显示 issue 列表，按 BLOCKED/REVIEW/INFO 分组。
  3. 对缺失 role、缺 Alpha、路径错误给可读中文。
  4. 禁止 BLOCKED 时继续创建 LayerSet。
- 验收：
  - invalid fixture 显示明确错误。
  - valid fixture 可进入映射页。
  - 文本不溢出紧凑面板。
- 回滚：删除 panel 和路由引用。
- 依赖：NRD-014。
- 估算：1d。

### NRD-016 — UI role 映射和 draw order 编辑

- 目标：允许用户修正 PSD/PNG 命名不匹配和层顺序。
- 写入：
  - `apps/desktop-ui/src/features/layerpack/LayerMappingEditor.tsx`
  - `apps/desktop-ui/src/features/layerpack/DrawOrderEditor.tsx`
  - `tests/ui/layerpack-mapping.test.mjs`
- 步骤：
  1. 列出每个 layer：文件名、role、zIndex、required。
  2. role 使用选择控件，zIndex 使用数字输入或上下移动。
  3. 检测重复 role/zIndex。
  4. 编辑只影响草稿，保存时重新预检。
- 验收：
  - 用户可把 `front_arm.png` 映射到 `upper_arm_front` 等 role。
  - 重复 zIndex 阻断。
  - 未保存草稿离开时提示。
- 回滚：移除 mapping editor，恢复只读 manifest。
- 依赖：NRD-015。
- 估算：1.5d。

### NRD-017 — UI pivot / socket 编辑

- 目标：让用户设置后续 Rig 需要的 pivot 和 `weapon-grip`。
- 写入：
  - `apps/desktop-ui/src/features/layerpack/PivotSocketEditor.tsx`
  - `tests/ui/layerpack-pivot-socket.test.mjs`
- 步骤：
  1. 在预览上显示当前层 pivot。
  2. 支持点击设置 pivot。
  3. 显示 primary weapon socket。
  4. 校验 weapon socket semantic 必须等于 StyleSpec 的 `weapon-grip`。
- 验收：
  - 缺 pivot 给 REVIEW 或 BLOCKED，按 policy 决定。
  - 缺 weapon-grip 必须 BLOCKED。
  - 编辑 socket 后 LayerSet 候选 hash 改变。
- 回滚：禁用 pivot/socket editor，要求 manifest 必填。
- 依赖：NRD-016。
- 估算：1.5d。

### NRD-018 — UI 合成预览和 side 对照

- 目标：让用户看到 LayerPack 合成结果是否接近 side reference。
- 写入：
  - `apps/desktop-ui/src/features/layerpack/CompositePreview.tsx`
  - `crates/adapters/src/layerpack/composite_preview.rs`
  - `tests/ui/layerpack-composite-preview.test.mjs`
- 步骤：
  1. Native/Rust 生成受限合成预览。
  2. UI 显示 side reference 和 composite。
  3. 显示 alpha/RGB/bbox 差异指标。
  4. 支持按 zIndex 重算预览。
- 验收：
  - 漏层/偏移 fixture 在 UI 中可见。
  - 预览图不超过 IPC 大小策略。
  - WebView 不直接读取原始 PNG。
- 回滚：移除合成预览，保留列表预检。
- 依赖：NRD-017。
- 估算：1.5d。

### NRD-019 — Layer Gate 审批闭环

- 目标：LayerPack 创建的 LayerSet 可以走人工批准。
- 写入：
  - `crates/application/src/layerpack/approve_layerpack_layers.rs`
  - `apps/desktop/src-tauri/src/ipc_host.rs`
  - `apps/desktop-ui/src/features/layerpack/LayerPackApprovalPanel.tsx`
  - `tests/integration/layerpack-approval.test.mjs`
- 步骤：
  1. `createLayerSet` 生成 PENDING LayerSet。
  2. `preview` 绑定当前 project revision、LayerPack hash、QA report hash。
  3. `approve` 使用原生确认框。
  4. 审批写入现有 gate，并使下游 Rig/动画按现有规则失效。
- 验收：
  - 无手绘操作也能批准 Layer Gate。
  - 修改任何 layer PNG/hash 后旧审批失效。
  - BLOCKED issue 存在时批准按钮不可用。
- 回滚：禁用 LayerPack approve 方法，保留预检。
- 依赖：NRD-018。
- 估算：1.5d。

### NRD-020 — 手绘工作台降级为 Repair Mode

- 目标：调整现有 UI 心智模型，避免用户误以为必须手绘。
- 写入：
  - `apps/desktop-ui/src/features/layers/LayerWorkspace.tsx`
  - `apps/desktop-ui/src/features/layers/layers.css`
  - `docs/user/quick-start-zh-CN.md`
- 步骤：
  1. 文案改为“手工修补/高级修补”。
  2. 新建标准分层清单按钮降为二级入口。
  3. 已有项目仍可打开旧 LayerSet。
  4. 当前手绘能力不删除。
- 验收：
  - 新用户主流程不出现“必须手绘”的暗示。
  - 老项目能继续打开和批准。
- 回滚：恢复旧文案和入口排序。
- 依赖：NRD-019。
- 估算：0.5d。

### NRD-021 — Coarse Rig 自动生成

- 目标：支持 7 层粗 Rig 快速跑通。
- 写入：
  - `crates/application/src/rig/layerpack_coarse_rig.rs`
  - `crates/domain/src/rig/candidate.rs`
  - `crates/application/tests/layerpack_coarse_rig.rs`
- 步骤：
  1. 将 body/head/arm/leg/weapon 映射到粗骨架。
  2. 使用 pivot/socket 生成默认骨骼位置。
  3. Rig candidate 标记 `COARSE_RIG`。
  4. 导出/预检中显示质量限制。
- 验收：
  - coarse fixture 能创建 Rig candidate。
  - 不能伪装成 standard profile。
  - weapon socket 仍绑定 `weapon-grip`。
- 回滚：禁用 coarse Rig 入口。
- 依赖：NRD-019。
- 估算：1.5d。

### NRD-022 — Standard Rig 自动生成

- 目标：从 17 层 standard LayerPack 生成标准 Rig 候选。
- 写入：
  - `crates/application/src/rig/layerpack_standard_rig.rs`
  - `crates/application/tests/layerpack_standard_rig.rs`
  - `apps/desktop-ui/src/features/rig/RigWorkspace.tsx`
- 步骤：
  1. 使用 role -> bone 映射。
  2. 使用 pivotPx 生成 slot pivot。
  3. 使用 socket config 生成 primary weapon socket。
  4. UI 显示“来自 LayerPack”的 Rig 来源。
- 验收：
  - standard fixture 生成完整 17 层 Rig。
  - 缺 pivot 或 socket 给结构化错误。
  - Rig approval 仍需人工确认。
- 回滚：恢复现有默认 Rig builder。
- 依赖：NRD-021。
- 估算：2d。

### NRD-023 — 导出 provenance 和兼容 manifest 更新

- 目标：导出包能说明分层来源为 LayerPack/PSD/Studio。
- 写入：
  - `crates/application/src/export/assembler.rs`
  - `crates/adapters/src/export/package.rs`
  - `docs/user/approval-and-export.md`
  - `tests/spine/spine42-contract.test.mjs`
- 步骤：
  1. 在 compatibility manifest 中记录 `layerSourceKind`。
  2. 记录 LayerPack manifest hash、layer PNG hashes、QA report hash。
  3. coarse rig 导出标记质量限制。
  4. 不输出用户绝对路径。
- 验收：
  - 导出包包含 LayerPack provenance。
  - `checksums.sha256` 覆盖新增 manifest。
  - 没有路径泄漏。
- 回滚：移除新增 provenance 字段，保留核心导出。
- 依赖：NRD-022。
- 估算：1d。

### NRD-024 — PSD 解析技术 Spike

- 目标：选择 PSD 解析实现前先做许可/兼容/体积评估。
- 写入：
  - `plan/NEWpic_reload/dev/reviews/PSD解析Spike.md`
  - `docs/compliance/F2S-PSD-DEPENDENCY-REVIEW.md`
  - `tools/spikes/layerpack/psd-reader-probe.*`
- 步骤：
  1. 比较 Rust/Node PSD 解析库。
  2. 检查许可证、维护状态、二进制体积。
  3. 用自制 PSD fixture 验证普通像素层、组、隐藏层、offset。
  4. 给出 adopt/reject 决策。
- 验收：
  - 没有审计结论前不新增发布依赖。
  - Spike 失败时 PSD 路线保持 EXTERNAL，PNG 路线不受影响。
- 回滚：删除 spike 脚本和草案。
- 依赖：NRD-023。
- 估算：1d。

### NRD-025 — PSD 普通层 adapter

- 目标：把普通 PSD 像素层转为 LayerPackDraft。
- 写入：
  - `crates/adapters/src/layerpack/psd_reader.rs`
  - `crates/adapters/tests/layerpack_psd.rs`
  - `docs/user/quick-start-zh-CN.md`
- 步骤：
  1. 只支持普通栅格图层。
  2. 读取图层名、可见性、像素、offset。
  3. 自动映射明显命名；模糊命名进入 UI 映射。
  4. 剪贴蒙版、图层效果、智能对象给可读错误。
- 验收：
  - 自制 PSD fixture 可导入。
  - 复杂 PSD 失败时提示“请导出透明 PNG 包”。
  - PSD source hash 进入 provenance。
- 回滚：禁用 PSD 输入策略，保留 PNG folder。
- 依赖：NRD-024。
- 估算：2d。

### NRD-026 — LayerPack Studio 草稿数据模型

- 目标：为无 PS 用户建立内置资源包制作草稿。
- 写入：
  - `crates/domain/src/layerpack/studio.rs`
  - `apps/desktop-ui/src/features/layerpack-studio/StudioWorkspace.tsx`
  - `tests/unit/layerpack-studio-model.test.mjs`
- 步骤：
  1. 定义 Studio draft：canvas、reference、imported parts、transform、role、zIndex、pivot/socket。
  2. 支持保存/恢复本地草稿。
  3. 不直接进入审批，必须 export 成 LayerPack 后重新预检。
- 验收：
  - 草稿不是 approved artifact。
  - 修改 transform 会改变 draft hash。
- 回滚：删除 Studio 入口和模型。
- 依赖：NRD-023。
- 估算：1.5d。

### NRD-027 — LayerPack Studio 拖拽摆放画布

- 目标：用户可拖入部件图并移动/缩放/旋转。
- 写入：
  - `apps/desktop-ui/src/features/layerpack-studio/StudioCanvas.tsx`
  - `apps/desktop-ui/src/features/layerpack-studio/StudioLayerList.tsx`
  - `apps/desktop-ui/src/features/layerpack-studio/studio.css`
  - `tests/ui/layerpack-studio-canvas.test.mjs`
- 步骤：
  1. 支持拖入透明 PNG。
  2. 支持选中、移动、缩放、旋转。
  3. 支持层级上下调整。
  4. 支持设置 role、pivot、weapon-grip。
- 验收：
  - 鼠标操作不改变原始导入文件。
  - transform 可撤销或至少可重置。
  - 非 Alpha 图片提示先透明化或拒绝。
- 回滚：隐藏 Studio Canvas，保留数据模型。
- 依赖：NRD-026。
- 估算：3d。

### NRD-028 — LayerPack Studio 导出资源包

- 目标：从 Studio draft 输出符合 schema 的 LayerPack 文件夹。
- 写入：
  - `crates/adapters/src/layerpack/studio_export.rs`
  - `apps/desktop/src-tauri/src/ipc_host.rs`
  - `apps/desktop-ui/src/features/layerpack-studio/StudioExportPanel.tsx`
  - `tests/integration/layerpack-studio-export.test.mjs`
- 步骤：
  1. 用户选择导出目录。
  2. Native 创建新空文件夹，不覆盖已有文件。
  3. 将每个部件渲染为全画布透明 PNG。
  4. 写入 `manifest.f2s-layerpack.json`。
  5. 立即可回到 NRD-014 导入向导预检。
- 验收：
  - 导出的包能通过 NRD-002 schema。
  - 导出的包能通过 NRD-019 Layer Gate。
  - 不写入 `%LOCALAPPDATA%\FlashToSpine` 私有数据根下的导出目录。
- 回滚：禁用 export 按钮，草稿仍保留。
- 依赖：NRD-027。
- 估算：2d。

### NRD-029 — 用户文档、模板和错误说明

- 目标：让用户知道应该准备什么、怎么命名、没有 PS 怎么办。
- 写入：
  - `docs/user/layerpack-guide-zh-CN.md`
  - `docs/user/quick-start-zh-CN.md`
  - `fixtures/layerpacks/template-standard-pack/**`
  - `fixtures/layerpacks/template-coarse-pack/**`
- 步骤：
  1. 写 standard/coarse 资源包模板。
  2. 写 PSD 路径、透明 PNG 路径、Studio 路径。
  3. 写常见错误：没有 Alpha、尺寸不一致、缺 weapon-grip、层为空、合成偏移。
  4. 明确版权和授权边界。
- 验收：
  - 用户可以按文档手工建立资源包。
  - 文档不承诺全自动拆图。
- 回滚：删除新文档和模板。
- 依赖：NRD-028。
- 估算：1d。

### NRD-030 — MVP 端到端回归

- 目标：证明从 LayerPack 到 Layer Gate/Rig 的最小链路可用。
- 写入：
  - `tests/integration/layerpack-workflow.test.mjs`
  - `tests/ui/layerpack-workflow.test.mjs`
  - `evidence/NEWPIC/README.md` 仅记录执行说明，不伪造结果
- 步骤：
  1. 使用 valid-standard fixture 创建项目。
  2. 导入 LayerPack。
  3. 创建 LayerSet candidate。
  4. 原生审批 mock 或测试 adapter 完成 Layer Gate。
  5. 创建 Rig candidate。
- 验收：
  - `npm test` 包含新用例。
  - 无外部 PS/AI/Spine 依赖也能运行核心测试。
  - CI 不提交执行产物。
- 回滚：移除 E2E 测试，不影响已实现功能。
- 依赖：NRD-029。
- 估算：1.5d。

### NRD-031 — 本地 AI 候选 Spike

- 目标：评估 See-through / SAM / anime-segmentation 是否值得作为候选生成器。
- 写入：
  - `plan/NEWpic_reload/dev/reviews/本地AI分层候选Spike.md`
  - `docs/compliance/F2S-LOCAL-SEGMENTATION-REVIEW.md`
  - `tools/spikes/layerpack/local-segmentation-probe.*`
- 步骤：
  1. 列出候选项目、许可证、模型权重来源、硬件要求。
  2. 用自制或授权测试图运行本地实验。
  3. 输出 LayerPack 候选质量报告。
  4. 明确默认关闭和 NOT_RUN/EXTERNAL 状态。
- 验收：
  - 没有模型时产品功能不受影响。
  - 任何 AI 输出都只能进入 candidate，不自动审批。
  - 审计未通过前不捆绑模型或运行时。
- 回滚：删除 spike 脚本和报告。
- 依赖：NRD-030。
- 估算：2d。

## 7. 建议 commit 切分

每个任务一个 commit。提交信息建议：

```text
newpic: add layerpack schema
newpic: add layerpack fixtures
newpic: add layerpack domain model
newpic: wire layerpack native import
newpic: add layerpack import workspace
newpic: support layerpack layer approval
newpic: add coarse rig from layerpack
newpic: add layerpack studio export
```

不要把 schema、Domain、Adapter、UI、测试全部塞进一个提交。若某任务超过 2 天，应拆成新的 `NRD-xxxA` 子任务再执行。

## 8. 验收总门

完成本计划后，必须满足：

1. 新用户无需在 FlashToSpine 内手绘遮罩，即可从透明 PNG LayerPack 完成 Layer Gate。
2. PSD 输入是可选增强，失败时能清晰降级为 PNG 包路径。
3. 无 PS/Krita/GIMP/Clip Studio 用户可使用 LayerPack Studio 制作资源包。
4. 重叠层不再天然失败；QA 以合成预览、role、draw order、pivot/socket 和人工审批为准。
5. 所有用户资产仍在本地；默认无公网请求。
6. 导出包能追踪 LayerPack manifest hash、layer hashes、QA report hash 和 Rig profile。
7. 旧项目和旧手绘修补链仍能打开，不被新入口破坏。
