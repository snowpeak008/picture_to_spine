---
doc_id: F2S-DOC-EXPORT-001
revision: 1.8
status: draft
canonical_for:
  - F2S-EXP
  - F2S-SPINE-CLI-001
  - F2S-SPINE-CLI-002
  - F2S-SPINE-CLI-003
  - F2S-SPINE-JSON-001
  - F2S-SPINE-JSON-002
  - F2S-SPINE-JSON-003
  - F2S-SPINE-JSON-004
  - F2S-GATE-EXPORT-001
depends_on:
  - F2S-DOC-GOV-001
  - F2S-DOC-REQ-001
  - F2S-DOC-CONTENT-001
  - F2S-DOC-ARCH-001
  - F2S-DOC-ENV-001
  - F2S-DOC-DOMAIN-001
  - F2S-DOC-STORE-001
  - F2S-DOC-ADR-001
review_score_ref: F2S-SCORE-DOC-EXPORT-001-R2
last_verified: 2026-07-11
---

# 13. Rig IR、PSD、PNG 与 Spine 4.2 导出设计

## 13.1 文档目标

本文定义 Production Assist 从内部 Rig IR 到可交付资产的唯一受支持路径，覆盖：

- 可重放、可迁移、可审计的 Rig IR；
- Spine-compatible PSD 与逐层 PNG；
- Spine Editor 4.2.43 JSON；
- 用户本地 Spine Professional CLI 的受控实验调用（取得本机证据前为 unverified）；
- 结构验证、语义 round-trip、失败隔离及兼容清单。

V1 首个正式兼容目标选定为 **仅 Spine Editor 4.2.43**。产品不捆绑 Spine Editor，也不捆绑任何官方 Spine Runtime；内部预览直接渲染 Rig IR。产品不会自行写入私有 `.spine` 格式，只有用户本地、合法安装的 `Spine.com` 可以在可选验证或转换步骤中创建 `.spine` 文件。

## 13.2 ADR 依据与导出边界

跨领域决策的唯一 canonical owner 是 `F2S-DOC-ADR-001`。本节只引用其已接受结论并派生导出约束；不得通过修改本文改变 ADR 状态、备选方案或退出条件。若摘要与 `F2S-DOC-ADR-001` 冲突，以 ADR 文档为准并先统筹整改。

### 依据 `F2S-ADR-ARCH-001`：Rig IR 唯一事实源

依照 `F2S-DOC-ADR-001`，所有编辑、自动分析、人工修正、撤销重做和导出均先提交为 Rig IR。本文据此规定 PSD、PNG、Spine JSON、预览缓存及 CLI 产物都是可再生派生物，不能反向成为项目真相源。

该依据使导出层规避：

- PSD 层名、Spine 名称和 UI 节点 ID 相互绑死；
- 不同导出格式的精度或默认值反向污染工程；
- CLI 中断后出现“部分成功”的项目状态；
- Spine 补丁升级迫使核心模型同步升级。

### 依据 `F2S-ADR-SPN-001`：精确版本

截至 2026-07-11，官方 4.2 最新稳定补丁为 `4.2.43`。生产配置固定如下：

```yaml
spineEditorVersion: 4.2.43
spineDataMajorMinor: "4.2"
allowLatestAlias: false
allowBeta: false
```

生产任务禁止使用 `latest`、`lateststable`、`latestbeta` 或 `4.2.xx`。本 Adapter 只接受4.2.43；目标版本变化先建立新ADR/新Adapter和迁移任务，本文不反向依赖下游恢复文档。

官方依据：

- [Spine Changelog](https://esotericsoftware.com/spine-changelog)
- [Spine Versioning](https://esotericsoftware.com/spine-versioning)
- [Spine CLI](https://esotericsoftware.com/spine-command-line-interface)

### 依据 `F2S-ADR-SPN-003`：不集成 Spine Runtime

桌面应用使用 PixiJS 8 渲染自有 Rig IR，不加载官方 Spine Runtime。Spine JSON 是边界输出，不是应用内部运行格式。这样既保证内部模型可控，也满足发布依赖许可证白名单。

## 13.3 组件关系

```text
React UI
   │ Tauri command / event
   ▼
Rust Application Service（唯一工作流事实源）
   ├── Rig IR Repository
   ├── Rig Validator
   ├── PSD Adapter
   ├── PNG Adapter
   ├── Spine 4.2 JSON Adapter
   ├── Export Orchestrator
   └── Local Spine CLI Gateway（可选）
             │ 无 shell、参数数组、仅本机
             ▼
       用户安装的 Spine.com 4.2.43

Python Worker
   └── 只通过 NDJSON 返回分析/候选结果；无项目目录写权限
```

导出采用 Ports and Adapters：领域层只认识 `RigProject` 和 `ExportRequest`，格式细节位于 Adapter，进程调用位于 CLI Gateway。

## 13.4 Rig IR 规范

### F2S-EXP-IR-001：顶层结构

建议使用带版本的 JSON 文档保存权威状态，并通过 Rust 类型和 JSON Schema 双重约束：

```json
{
  "schemaVersion": "1.0.0",
  "projectId": "01900000-0000-7000-8000-000000000001",
  "revision": 42,
  "coordinateSystem": "x-right-y-up-ccw-degrees",
  "pixelsPerUnit": 100,
  "timeBase": { "numerator": 1, "denominator": 30000 },
  "skeleton": {},
  "assets": {},
  "skins": {},
  "animations": {},
  "events": {},
  "exportProfiles": {}
}
```

领域对象至少包括：

| 稳定对象 | 必要字段 | 约束 |
|---|---|---|
| `RigProject` | schema、revision、坐标系 | revision 单调递增 |
| `Bone` | UUID、name、parent、setup transform | 父骨必须先存在，不得成环 |
| `Slot` | UUID、name、bone、drawOrder | 名称在 skeleton 内唯一 |
| `Attachment` | UUID、slot、kind、assetHash、pivot | kind 为 region/mesh/bounding-box/point/clipping |
| `Mesh` | vertices、uv、triangles、weights | 索引合法；每顶点权重归一化 |
| `Skin` | UUID、name、slot/attachment map | default skin 永久存在 |
| `Animation` | UUID、name、`durationTick:string(i64-decimal)`、timelines | tick字符串规范、非负、有序且不超过durationTick |
| `Event` | UUID、name、payload schema | 命中帧须经人工确认 |
| `AssetRef` | SHA-256、mediaType、width、height | 内容寻址、不可就地改写 |

UUID 只用于内部稳定引用；人类可读名称独立保存。重命名不得改变 UUID。Spine JSON Adapter 负责把 UUID 引用解析为确定性名称，不能把显示名当数据库主键。

### F2S-EXP-IR-002：坐标、角度与精度

- 内部坐标：`X` 向右、`Y` 向上；单位为像素浮点数。
- 内部角度：度数、逆时针为正。
- PNG/PSD 像素坐标：左上为原点、`Y` 向下；只在图像 Adapter 边界转换。
- 变换计算使用 `f64`；写入 Spine JSON 时按测试确认的精度输出，禁止无说明的整数化。
- Pivot、骨骼原点、裁切偏移必须保留原始浮点值。
- 同一 Attachment 的图像替换不得隐式改变 pivot。

### F2S-EXP-TIME-001：整数 tick 与 Spine 秒数边界

本节实现 canonical `F2S-ADR-TIME-001`，不得扩张或改写其时间政策：

- Rig IR 顶层保存规范正有理时间基 `timeBase={numerator:u32,denominator:u32}`；两项是大于零的 JSON integer 且最大公约数必须为1，默认唯一值为 `1/30000` 秒/tick。零值、非约分形式或u32越界均拒绝载入。
- 动画时长、关键帧、事件与 marker 的 tick 必须以 JSON string 保存，词法为 `0|[1-9][0-9]*`，解码后不超过i64最大值；禁止JSON number、前导零、加号和`-0`。时间线tick单调不减且不超过`durationTick`。
- 外部秒数只接受最长64字节、无指数的规范十进制文本。Adapter以精确十进制有理数、checked i128中间值和round-half-to-even计算tick，保存原文本、结果与精确舍入差；领域模型、事件日志和快照禁止持久化`f32/f64`时间。
- V1单动画精确时长限定为0至60秒。Spine 4.2.43 JSON Adapter用checked i128计算`tick*numerator/denominator`，再half-even至最多9位小数；JSON number去尾零、禁指数、`-0`归一化为`0`，且绝不回写Rig IR。
- 每条timeline同时检查格式化文本与解析后的IEEE-754 binary32：值必须有限、非负、保持原tick顺序；任意两个不同tick在文本或binary32层相等/逆序均失败，并报告timeline、两个tick、时间基、文本和binary32 bit pattern。无本地Editor时该检查是依据官方4.2 Runtime `getFloat`行为的保守代理；只有4.2.43 CLI实际往返可把外部兼容状态升级为VERIFIED。
- `F2S-TST-PROP-002`验证全i64字符串往返与规范有理数，`F2S-TST-CONTRACT-004`验证跨JSON/IPC的schema和half-even边界，`F2S-TST-CONTRACT-005`验证Adapter安全算术/格式化/binary32碰撞，`F2S-TST-GOLD-004`验证4.2.43往返时间语义。

### F2S-EXP-IR-003：名称规范

Spine 导出名采用 ASCII 稳定名：`[a-z][a-z0-9_/-]{0,63}`。UI 可维护独立中文显示名。非法名称由用户确认后修复，不能在导出时静默重命名。

名称映射保存到 `exports/spine-4.2.43/name-map.json`，包含内部 UUID、显示名、导出名和冲突处理记录。

### F2S-EXP-IR-004：核心不变量

导出前必须全部满足：

1. 骨骼图无环且只有一个约定 root。
2. 所有 Slot 引用有效 Bone。
3. 所有可见 Attachment 引用存在且可解码的资产。
4. Mesh triangle、UV 和 weight 索引均在范围内。
5. 每个顶点的有效权重和在容差内等于 1。
6. 时间基有效；所有动画、关键帧、事件与 marker 使用 `i64` tick，且时间线 tick 单调、不超过 `durationTick`；Spine 秒数转换符合 `F2S-EXP-TIME-001`。
7. Slot draw order 无重复、无空洞。
8. 关键动作的命中事件、脚底接触与武器挂点已经人工审批。
9. 未批准的 AI 候选结果不能进入 release export。
10. Candidate profile 使用的字段必须已登记且不为 `UNSUPPORTED`；Release profile 使用的每项能力必须在 Spine 4.2.43 功能支持矩阵中标记为 `VERIFIED`。

## 13.5 PSD 输出

### F2S-EXP-PSD-001：定位

PSD 是可人工修订和可由 Spine Editor UI 导入的交换格式，不是权威工程文件。工具输出 PSD，但不自动操纵 Photoshop，也不假定 Spine CLI 支持 PSD 导入。

### F2S-EXP-PSD-002：兼容配置

- 8-bit RGBA、sRGB；禁止 CMYK。
- 图层必须包含实际像素；Adjustment Layer、未栅格化 Layer Style 等不能作为唯一视觉来源。
- 每个 Attachment 使用独立像素层或明确的 `[merge]` 组。
- 保留透明边缘和关节隐藏区，不对源图层破坏性裁边。
- 首条横/竖参考线或 `[origin]` 层定义 `0,0`。
- 层顺序必须与 Rig IR 初始 draw order 一致。
- 保存时生成 `psd-manifest.json`，记录层到 Attachment UUID 的映射。

P0 写出契约由版本化 `PsdExportProfile` 冻结，不能仅靠约定层名：

```json
{
  "schemaVersion": "1.0.0",
  "profileId": "spine-4.2.43-minimal-layered",
  "writerAdapter": "ag-psd",
  "canvas": {"width": 2048, "height": 2048, "dpi": 72},
  "pixelFormat": {"depth": 8, "channels": "RGBA", "colorSpace": "sRGB", "alpha": "straight"},
  "originEncoding": "guides-and-origin-layer",
  "preserveGroups": true,
  "preserveVisibility": true,
  "preserveOpacity": true,
  "trimPolicy": "preserve-source-bounds",
  "unsupportedFeatures": "report-and-reject-if-p0-loss"
}
```

`writerAdapter` 是经 M00 Spike 冻结后的实现标识；`ag-psd` 只是当前 MIT 候选。若采用其 Web Worker 实现，它只能接收尺寸受限的层像素/DTO并返回 PSD byte stream + manifest，不获得项目路径或文件权限；Rust 仍负责独立 reopen、hash、staging 写入和最终提交。候选 writer 未通过许可证、IPC/内存峰值、独立 reopen 和 Spine 4.2.43 UI 导入夹具前，PSD 能力保持 `UNVERIFIED`。

`psd-manifest.json` 是 PSD 的可审计语义伴随文件，至少包含：

```json
{
  "schemaVersion": "1.0.0",
  "documentSha256": "sha256:...",
  "profileHash": "sha256:...",
  "canvas": {"width": 2048, "height": 2048, "dpi": 72},
  "origin": {"x": 1024.0, "y": 1536.0, "encoding": "guides-and-origin-layer"},
  "nodes": [{
    "nodeId": "uuid",
    "parentNodeId": "uuid-or-null",
    "kind": "group|pixel|origin",
    "attachmentId": "uuid-or-null",
    "name": "arm_front",
    "taggedName": "[slot:arm_front] arm_front",
    "siblingIndex": 3,
    "drawOrder": 7,
    "visible": true,
    "effectiveVisible": true,
    "opacity": 255,
    "effectiveOpacity": 255,
    "blendMode": "normal",
    "pixelAssetHash": "sha256:...",
    "bounds": {"x": 10, "y": 20, "width": 300, "height": 500},
    "trimOffset": {"x": 0, "y": 0},
    "pivot": {"x": 120.5, "y": 260.25},
    "alphaMode": "straight",
    "pngLogicalPath": "body/arm_front.png"
  }],
  "degradations": []
}
```

组和像素层均有稳定 `nodeId`；`parentNodeId + siblingIndex` 完整恢复组/层级和顺序。`visible/opacity` 保存节点自身值，`effective*` 保存继承后的结果。`origin`、`bounds`、`trimOffset` 和 `pivot` 的坐标系必须在 Schema 中固定，禁止凭 PSD 裁边结果反推。

允许的 Spine 标签由 Adapter 白名单生成，例如：

```text
[origin]
[bone:torso]
[slot:arm_front]
[skin:default]
[mesh]
[path:body/arm_front]
[pad:8]
```

不得生成未经 4.2.43 验证的标签。标签、层级和限制以[官方 PSD Import 文档](https://esotericsoftware.com/spine-import-psd)为准。

### F2S-EXP-PSD-003：reopen 与字段级断言

每个 P0 PSD fixture 必须执行三层验证：

1. writer 写出后，以独立、锁版且通过许可门的只读解析器 reopen；当前测试候选为 MIT `psd-tools`，不得与 writer 共用同一反序列化实现来冒充独立验证。
2. 逐节点比较 canvas、组/层级、名称、`parentNodeId`、`siblingIndex`、可见性、opacity、blend mode、像素边界、逐层 alpha、origin 和 pivot/trim 映射；任何缺项均为 `fail`，不能降为 warning。
3. 重新合成图与批准母版做 alpha-aware 像素比较，并在 Spine Editor 4.2.43 UI 中人工导入，核对 origin、slot/skin/path 标签、PNG 路径和 draw order。证据绑定 PSD/manifest/profile/reader/Editor 版本哈希。

高级 Adjustment、Smart Object、未栅格化 Layer Style 等只可列入 `degradations`；若降级导致上述 P0 字段或可见像素丢失，整项 PSD 输出失败。

### F2S-EXP-PSD-004：PSD 导入边界

官方公开 CLI 文档只列出 JSON、binary 和项目导入，没有公开 PSD CLI 导入参数。因此：

- PSD → Spine 是用户在 Spine Editor 4.2.43 中执行的 UI 工作流；
- 自动化 round-trip 使用 Spine JSON，不使用隐藏或逆向参数；
- 产品 UI 提供“打开 PSD 所在目录”和“显示 Spine 导入步骤”，不模拟鼠标键盘；
- PSD 手工导入结果作为验收证据，但不作为无人值守构建步骤。

## 13.6 PNG 输出

### F2S-EXP-PNG-001：源附件 PNG

- 使用 8-bit RGBA PNG、sRGB 色彩空间。
- 源附件使用 straight alpha；是否生成 PMA atlas 由后续导出 profile 明确决定。
- 默认至少保留 4～8 px 透明 padding；具体值进入 ExportProfile。
- 文件内容以 SHA-256 寻址，逻辑路径由 manifest 映射。
- 禁止把空白图、零尺寸图、完全透明但非占位用途的图输出为成功。
- 重采样必须记录算法、源尺寸、目标尺寸和色彩空间。

### F2S-EXP-PNG-002：目录结构

```text
exports/spine-4.2.43/<export-id>/
├── images/
│   ├── body/torso.png
│   ├── body/arm_front.png
│   └── weapon/primary_weapon.png
├── character.spine.json
├── character.psd
├── name-map.json
├── compatibility-manifest.json
├── validation-report.json
└── checksums.sha256
```

路径使用 `/` 作为 manifest 逻辑分隔符；Windows 实际路径由 Rust Path API 转换。禁止手工字符串拼接路径。

## 13.7 Spine 4.2.43 JSON Adapter

### F2S-SPINE-JSON-001：输出契约

输出根节点至少包含经过验证的 `skeleton`、`bones`、`slots`、`skins` 和 `animations`。版本元数据固定：

```json
{
  "skeleton": {
    "spine": "4.2.43",
    "images": "./images/"
  }
}
```

官方格式文档允许其他工具生成可被 Spine 导入的 JSON，但当前公开页面仍包含较早版本示例，不能被当作 4.2 全功能机器 Schema。Adapter 必须以官方文档、4.2.43 Editor 黑盒 round-trip 和合法生成的 golden fixtures 三者共同验证。[Spine JSON Format](https://esotericsoftware.com/spine-json-format)

### F2S-SPINE-JSON-002：受支持特性登记

证据状态与产品可用模式分开登记。每个特性必须有以下证据状态之一：

- `UNVERIFIED`：字段映射已实现或计划实现，但尚无完整 4.2.43 证据；只可进入 Candidate profile；
- `VERIFIED`：已通过结构、4.2.43 CLI round-trip 和适用的视觉/语义测试；
- `UNSUPPORTED`：导出前硬错误；
- `NOT_APPLICABLE`：当前产品不会产生该特性。

初始策略：

| 特性 | 初始策略 |
|---|---|
| Bone/Slot/Region Attachment | 初始 `UNVERIFIED`；L0/L1 后可用于 Candidate，完整证据后晋级 |
| Mesh/Weights/Deform | 初始 `UNVERIFIED`；完成 Professional round-trip 后晋级 |
| Skin/Attachment swap | 初始 `UNVERIFIED`；完成 round-trip 后晋级 |
| IK/Transform constraint | 初始 `UNVERIFIED`；完成 Professional round-trip 后晋级 |
| Bounding box/Point/Event | 初始 `UNVERIFIED`；完成人工语义验收后晋级 |
| Sequence | 初始 `UNVERIFIED`，V1 Release profile 默认禁用 |
| Physics constraint | 初始 `UNSUPPORTED` |
| 未知未来字段 | 硬错误，不透传 |

不允许为了“导出成功”静默丢弃不支持的 Mesh、权重、约束、事件或时间线。

每条能力记录包含 `featureId`、状态、Adapter 版本、Rig IR Schema、Editor patch、fixture hash、测试/证据 ID、批准人和时间。只有 `F2S-GATE-EXPORT-001` 接受的证据可执行 `UNVERIFIED → VERIFIED`；Adapter、Schema、目标 patch、fixture 预期或比较器发生语义变化时自动退回 `UNVERIFIED`。状态不得由 UI、配置文件或一次成功退出码直接提升。

### F2S-SPINE-JSON-003：确定性

相同 Rig IR revision 与 ExportProfile 必须产生字节稳定或语义稳定的输出：

- 数组使用领域定义的稳定顺序；
- 对象键使用固定序列化规则；
- 浮点输出不受系统区域设置影响；
- 时间戳、绝对路径和随机 ID 不进入 JSON 内容；
- 导出 ID 根据输入 revision、profile 和 Adapter 版本计算；
- 重复运行不得产生新的项目 revision。

### F2S-SPINE-JSON-004：Candidate 与 Release profile

| Profile | 最低条件 | 允许状态 | 产物状态 |
|---|---|---|---|
| `candidate` | L0/L1、字段已登记、无 `UNSUPPORTED`/未知字段、全部降级可见 | 可含 `UNVERIFIED` | `exported_unverified`；不得宣传 Spine 兼容已验证 |
| `release` | 全部内容审批有效；所有实际使用特性 `VERIFIED`；L0–L3通过 | 仅 `VERIFIED/NOT_APPLICABLE` | `spine_verified`，并绑定证据清单 |

两种 profile 使用不同的 profile hash 和 UI 动作。Candidate 不能通过改文件名、复制目录或接受 warning 晋级为 Release；晋级必须重新执行 preflight、适用 round-trip 和人工审批。

## 13.8 实验性本地 Spine Professional CLI Gateway

2026-07-11（Asia/Shanghai）的当前环境审计检查了PATH以及`C:\Program Files\Spine`、`C:\Program Files (x86)\Spine`、`%LOCALAPPDATA%\Programs\Spine`三个约定位置，均未发现`Spine.com`；授权状态也无可验证证据，记录为`F2S-OQ-SPN-001 / EXTERNAL_UNVERIFIED`。该有限范围检查不证明机器其他位置不存在用户安装。文档中的参数来自官方说明，不等于已在目标机器成功运行。只有用户显式选择合法可执行文件，并完成 `F2S-TST-CONTRACT-005`、`F2S-TST-GOLD-001`、`F2S-TST-GOLD-002`、`F2S-TST-GOLD-003`、`F2S-TST-GOLD-004`、`F2S-TST-GOLD-005`、`F2S-TST-GOLD-006` 与 `F2S-EVD-M08-007` 本机归档后，才能把对应能力改为 `VERIFIED`。

### F2S-SPINE-CLI-001：外部依赖发现

用户在设置页显式选择 `Spine.com`。程序只保存规范化路径，不复制文件，不读取安装目录中的激活信息。首次使用和每次路径变化时执行：

1. 路径存在且文件名符合 Windows 预期。
2. 目标不是项目目录内可被素材替换的可执行文件。
3. 先仅执行离线版本探测 `Spine.com --version`，确认当前选中版本是否为 4.2.43；该 probe 只用于能力提示，不作为随后转换进程的版本证明。
4. 用户确认其拥有合法的 Spine Professional 或适用的 Enterprise 许可。
5. CLI 能力探测结果写入本机设置，不写入可分享项目。

默认离线模式禁止为版本探测隐式使用 `--update`，因为该参数可能触发版本查询或下载。只有在版本探测不匹配、用户查看联网目标和影响并显式批准“让本地 Spine 切换/获取 4.2.43”后，才可单独执行一次 `Spine.com --update 4.2.43 --version`。程序不能自动下载 Spine、不能保存激活码，也不能代用户接受 EULA。

### F2S-SPINE-CLI-002：命令执行安全

- Rust 使用 `std::process::Command` 或 Tauri sidecar 等价的参数数组 API。
- 禁止 `cmd /c`、PowerShell、shell 字符串或 `eval`。
- 可执行路径与每个参数分别传递。
- PinnedValidation/import/export的输入、输出均限制在当前`.f2sproj`的内部staging/export目录；Spine CLI永不直接写用户选择的项目外publish root。外部目录只能在canonical snapshot提交后由`PublishAttempt` Adapter接收受校验copy。
- 路径 canonicalize 后检查 junction、符号链接和越界。
- 不接受从动作描述、图片元数据或 AI 输出直接形成 CLI 参数。
- 每个命令有超时、取消、输出大小上限和唯一 operation ID。
- stdout/stderr 做隐私清洗后写日志；绝不记录激活码或用户邮件。
- 参数 policy 分操作冻结：probe 只允许 `--version`；PinnedValidation 允许固定字面量 `--update 4.2.43` 与受控 `--input/--output/--import/--export/--clean`；显式准备操作只允许 `--update 4.2.43 --version`。`--last-export-settings` 仅用于用户确认的设置提取，不能进入验证基线。是否允许联网由独立 network grant 决定，不能由出现 `--update` 自动获得。
- 所有 CLI 操作先取得当前用户范围的 `SpineCliLease`（命名 mutex + operation ID），应用自身不得并发 probe/import/export/update；mutex 不能约束用户手工打开的 Spine，因此仍必须做转换后版本证明。
- 每个真正参与验证的 import/export 进程必须显式包含 `--update 4.2.43` 作为版本选择参数。首次获取/切换该 patch 前必须得到一次明确联网批准；普通转换阶段由网络策略阻断 Spine egress，若本机没有已缓存的合法 4.2.43 而命令尝试联网，则任务失败为 `UNVERIFIED`，不得自动放行网络。
- 退出码 0 不是版本证明。Rust 必须解析重导 JSON 的 `skeleton.spine`、脱敏 CLI 版本输出和产物哈希；任一缺失或不是精确 `4.2.43` 时隔离整个 operation。

### F2S-SPINE-CLI-003：实验命令模板

每次转换前运行并记录脱敏 probe，但 probe 与转换后证明承担不同职责：

```text
Spine.com --version
```

只有 probe 明确返回 4.2.43，才进入本轮转换。转换进程仍显式钉住 patch；网络是否允许由独立策略控制，不能通过省略版本参数换取“离线”：

JSON 创建临时 `.spine` 项目：

```text
Spine.com
  --update 4.2.43
  --input <staging/character.spine.json>
  --output <staging/roundtrip/character.spine>
  --import <stable-skeleton-name>
```

从临时项目重导 JSON：

```text
Spine.com
  --update 4.2.43
  --input <staging/roundtrip/character.spine>
  --output <staging/roundtrip/reexport>
  --export <approved-export-settings.json>
```

若 probe 不是 4.2.43，转换必须停止。用户可在独立确认界面批准以下可能联网的准备命令，完成后重新执行纯 `--version` probe：

```text
Spine.com --update 4.2.43 --version
```

Windows 优先使用 `Spine.com`，因为官方说明它会把输出写到控制台并等待进程结束。图片/视频导出需要窗口系统及 OpenGL，不能假定服务器 headless 环境可用。命令模板在取得真实安装、授权、退出码和 round-trip 证据前不得称为已支持。

每轮验证的 `SpineCliEvidence` 至少包含 executable canonical path 的哈希、mutex lease ID、参数模板哈希、probe 输出摘要、每个进程起止时间/退出码、重导 JSON 哈希及其中的实际 `skeleton.spine`。实际 patch 证明来自本轮转换产物，不从上一次 probe 或用户设置继承。

## 13.9 Round-trip 验证

### F2S-GATE-EXPORT-001：四级验证

| 级别 | 验证 | 没有 Spine 时 |
|---|---|---|
| L0 | JSON Schema、领域不变量、图片/PSD字段级reopen | 必须通过 |
| L1 | Rig IR → JSON → 内部只读解析器语义一致 | Candidate/Release 均必须通过 |
| L2 | 实验性 Spine 4.2.43 CLI 导入 → `.spine` → 重导 JSON | 无本机证据时标记 `not-run`，不能伪装通过 |
| L3 | Spine Editor 4.2.43 人工打开、视觉检查和关键动作审批 | release candidate 必须通过 |

### F2S-EXP-RT-002：语义比较

CLI 重导可能改变排序、默认字段和浮点表现，不能使用原始文件字节比较。比较器先 canonicalize，再检查：

- 骨骼父子关系、setup transform；
- Slot 所属骨骼和 draw order；
- Skin、Attachment 与图片路径；
- Mesh 顶点、三角形、UV、权重容差；
- Animation 名称、时长、时间线和曲线；
- Event 名称、时间和 payload；
- 不允许出现未登记的数据丢失。

每项结果为 `pass`、`fail` 或 `not-run`。最终状态不允许把 `not-run` 汇总为 `pass`。

### F2S-EXP-RT-003：兼容清单

每次导出写入：

```json
{
  "editorVersion": "4.2.43",
  "dataVersion": "4.2",
  "adapterVersion": "1.0.0",
  "rigIrSchemaVersion": "1.0.0",
  "sourceRevision": 42,
  "exportProfileHash": "sha256:...",
  "psdProfileHash": "sha256:...",
  "cli": {
    "used": true,
    "capabilityStatus": "UNVERIFIED",
    "pathStored": false,
    "reportedVersion": "4.2.43",
    "versionProbe": "pass",
    "networkVersionSwitchApproved": false
  },
  "exportMode": "release",
  "validation": {
    "l0": "pass",
    "l1": "pass",
    "l2": "pass",
    "l3": "pass"
  }
}
```

绝对路径、用户名、机器名和许可证信息不得进入可分享清单。

## 13.10 导出事务与状态机

下列名称是 `ExportStage` 进度投影，不是新的持久 Job 状态；Job 仍只使用 `F2S-DOC-DOMAIN-001` 的 canonical 状态和终态仲裁。stage 按 profile 分流：

```text
queued → validating → materializing → adapter-writing
      → optional-cli-roundtrip → comparing → awaiting-human-approval
      → committed
```

Candidate 在 L0/L1 后可跳过 CLI/L3，但只能提交为 `exported_unverified`；Release 不得跳过 `optional-cli-roundtrip`（对 Release 实际为 required）和 L3。状态机中的 `committed` 表示导出快照原子提交，不等价于 `spine_verified`。

失败或取消进入 `failed`/`cancelled`，不得直接从中间状态标记 `committed`。所有输出先写入：

```text
.f2sproj/work/staging/<operation-id>/
```

全部验证通过后，以同卷原子rename提升到项目内`exports/<export-id>`，该完整目录是canonical export snapshot。失败staging按`F2S-DOC-STORE-001`的OperationJournal、quarantine与清理契约处理，不能覆盖上一个成功导出；17号恢复文档只消费该记录，不反向改变本事务。

项目外路径只属于后置`PublishAttempt` Adapter：它从已提交内部snapshot读取，在用户批准的外部root内创建同卷临时目录，逐文件copy/flush/hash后rename为新版本目录。禁止把项目staging跨卷rename或让外部目录成为唯一产物。publish失败不改变ExportStage=`committed`，单独记录失败、残余临时目录和重试依据；重试始终从内部snapshot开始。

## 13.11 UI 交互要求

- 导出面板明确显示目标为 Spine Editor 4.2.43。
- 把“生成 Spine JSON”和“通过本地 Spine 验证”显示为两个步骤。
- 未配置 CLI 时仍允许以 Candidate profile 导出 PSD/PNG/Rig IR/候选 JSON，但显示 `exported_unverified`；Release profile 必须禁用并显示缺失证据。
- 所有会导致丢字段的情况必须阻断，不提供“忽略并继续”默认按钮。
- 展示功能支持矩阵和每个 `not-run` 原因。
- 人工审批记录操作者、本地时间、Rig revision 和预览哈希。
- “在 Spine 中打开”只启动用户选择的可执行文件和明确文件，不注入脚本。

## 13.12 测试与验收

### 固定测试资产

至少建立以下合法自有 fixtures：

1. 单 Bone + 单 Region 最小角色。
2. 完整二次元侧视类人骨架。
3. Weighted Mesh 与 Deform。
4. Skin 和手型/武器 Attachment swap。
5. IK、武器挂点、Bounding box 和 Event。
6. idle/run/jump/fall/dash/attack×3/hit/death 全动作集合。
7. 中文路径、空格路径、超长路径和只读目录案例。
8. 故意损坏的 Mesh、缺图、成环骨骼和非法时间线。

### 稳定验收映射

- `F2S-AC-EXP-001`：同一 revision 与 profile 连续导出两次，Rig IR、PNG/PSD 语义和 JSON 输出一致；不支持特性必须硬错误且不得静默丢失。
- `F2S-FR-EXP-002`：PSD reopen 逐字段证明 canvas、组/层级、名称、顺序、可见性、opacity/alpha、origin/pivot、像素层与 manifest 一致；任一 P0 字段缺失即失败。
- `F2S-AC-EXP-002`：所有 release JSON 明确声明 `4.2.43`；合法 Professional CLI 的 L2 round-trip 对已支持 fixture 通过，并记录脱敏命令摘要、退出码和产物哈希。
- `F2S-AC-EXP-003`：没有 Spine、版本不符或 CLI 失败时，核心编辑和 Rig IR 预览继续工作，产物只可标记 `exported_unverified` 或 `incompatible`。
- `F2S-NFR-SEC-003`：包含空格、中文和 shell 元字符的命令参数不会造成命令注入。
- `F2S-ADR-LIC-001` / `F2S-LIC-POLICY-001`：发布包扫描确认不包含 Spine Editor、激活信息或官方 Spine Runtime，并以 canonical policy snapshot 为判定依据。

## 13.13 明确不做

- 不逆向或直接写 `.spine` 私有项目格式。
- 不捆绑、下载或静默安装 Spine Editor。
- 不捆绑官方 Spine Runtime。
- 不支持 Spine 4.3 或任何 beta 数据。
- 不调用未公开的 PSD CLI 参数。
- 不把 AI 推断的命中帧直接视为已审批游戏数据。
- 不承诺任意 Spine JSON 都能无损导入；仅承诺已登记并通过 4.2.43 round-trip 的子集。
