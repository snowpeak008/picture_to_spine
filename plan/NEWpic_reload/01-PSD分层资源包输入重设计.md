---
doc_id: F2S-NEWPIC-RELOAD-001
revision: 0.1
status: draft
created_at: 2026-07-12
canonical_for:
  - F2S-NEWPIC-INPUT-001
  - F2S-NEWPIC-LAYERPACK-001
  - F2S-NEWPIC-PSD-001
  - F2S-NEWPIC-QA-001
depends_on:
  - F2S-DOC-CONTENT-001
  - F2S-DOC-PIPE-001
  - F2S-DOC-DOMAIN-001
  - F2S-DOC-EXPORT-001
  - F2S-DOC-SEC-001
---

# PSD / 分层资源包输入重设计

## 1. 背景与结论

当前分层路线以“单张母版图 + 用户在 WebView 内手绘遮罩”为主。实际测试已经暴露出关键问题：对普通用户来说，手绘遮罩难度高、反馈慢、容易把画面涂乱；对生产资产来说，严格的“每个可见像素必须只属于一个层”也不符合 Spine / Live2D / 2D 骨骼动画的常见素材组织方式。

新的设计结论是：

1. 将 **PSD / 分层透明 PNG 资源包** 升级为主输入路径。
2. 将当前手绘遮罩降级为 **高级修补工具**，不再作为默认生产流程。
3. 允许层之间存在受控重叠，以 draw order、pivot、socket 和合成预览作为质量基准。
4. 单张母版图只作为身份、风格、合成对照和参考图；真正进入 Rig 的资源来自分层包。
5. 若用户没有 Photoshop，也应能通过 Krita / GIMP / Clip Studio / 内置轻量 LayerPack Studio / 本地 AI 分割候选产出资源包。

## 2. 目标与非目标

### 2.1 目标

- 定义可导入、可校验、可审批、可追踪的 LayerPack 文件夹规范。
- 支持 PSD 导入和透明 PNG 文件夹导入两条主路径。
- 支持三视图参考图命名规则，保留 front / side / back 对照能力。
- 支持粗粒度和标准粒度两种 Rig profile。
- 允许受控重叠，改用合成差异和层级语义做 QA。
- 为无 PS 用户设计可落地的资源包产出方案。
- 保持现有项目原则：本地优先、无公网图片 API、人工审批、CAS 哈希绑定。

### 2.2 非目标

- 不在 V1 内实现完整 Photoshop 替代品。
- 不承诺从任意单张图片自动拆出完美可动画层。
- 不把公网抠图、生图或第三方 SaaS 接入默认生产路径。
- 不绕过母版、分层、Rig、动作关键姿势等人工审批门。
- 不因引入 PSD / PNG 包而放宽版权、授权和许可边界。

## 3. 输入模型重定义

### 3.1 三类输入资产

| 类型 | 作用 | 是否进入 Rig | 说明 |
| --- | --- | --- | --- |
| `reference-view` | 角色身份、比例、三视图对照 | 否 | front / side / back 等参考图，不直接生成 slot |
| `layer-source` | 分层源文件 | 是 | PSD 或多个透明 PNG |
| `master-composite` | 合成对照图 | 否 | 用于 QA，比较分层重组是否接近目标效果 |

### 3.2 推荐用户准备内容

最推荐的输入不是一张完整图，而是一个资源包文件夹：

```text
tifa_test__f2s_layerpack__v001/
  manifest.f2s-layerpack.json
  views/
    tifa_test__view_front__v001.png
    tifa_test__view_side__v001.png
    tifa_test__view_back__v001.png
  sources/
    tifa_test__layers__v001.psd
  layers/
    standard/
      010_hair_back.png
      020_body.png
      030_head.png
      040_hair_front.png
      110_upper_arm_back.png
      120_forearm_back.png
      130_hand_back.png
      210_upper_arm_front.png
      220_forearm_front.png
      230_hand_front.png
      310_thigh_back.png
      320_shin_back.png
      330_foot_back.png
      410_thigh_front.png
      420_shin_front.png
      430_foot_front.png
      900_weapon.png
  preview/
    tifa_test__composite_side__v001.png
```

PSD 是可选的。如果存在 PSD，工具优先读取 PSD 图层；如果用户只提供 `layers/standard/*.png`，也可以直接导入。

## 4. 文件夹与文件命名规范

### 4.1 资源包文件夹名称

格式：

```text
<character_slug>__f2s_layerpack__v<NNN>
```

示例：

```text
tifa_test__f2s_layerpack__v001
black_hair_fighter__f2s_layerpack__v003
```

规则：

- `character_slug` 只允许小写 ASCII、数字、短横线和下划线：`[a-z0-9_-]+`。
- 版本号固定三位数字：`v001`、`v002`。
- 文件夹内必须有 `manifest.f2s-layerpack.json`。
- 用户可在 manifest 的 `displayName` 写中文名；文件名保持机器友好。

### 4.2 三视图参考图命名

格式：

```text
<character_slug>__view_<view_key>__v<NNN>.<ext>
```

必需视图：

```text
<slug>__view_side__v001.png
```

推荐视图：

```text
<slug>__view_front__v001.png
<slug>__view_side__v001.png
<slug>__view_back__v001.png
```

可选视图：

```text
<slug>__view_front_45__v001.png
<slug>__view_back_45__v001.png
<slug>__view_side_weapon_hidden__v001.png
```

约束：

- V1 Rig 仍以 `side` 为主视图。
- `front` 和 `back` 只作身份、比例、服装结构参考。
- 参考图可为 PNG / JPEG / WebP；进入层的图片必须是 PNG 且带 Alpha。

### 4.3 PSD 文件命名

格式：

```text
<character_slug>__layers__v<NNN>.psd
```

示例：

```text
black_hair_fighter__layers__v001.psd
```

约束：

- PSD 内图层名允许用户使用自然语言，但导入时必须映射到规范 role。
- PSD 解析失败时，不得自动降级为整图手绘；应提示用户导出透明 PNG 包。

### 4.4 PNG 层文件命名

标准 profile 使用排序前缀 + role：

```text
<z_order>_<role>.png
```

示例：

```text
010_hair_back.png
020_body.png
030_head.png
040_hair_front.png
110_upper_arm_back.png
120_forearm_back.png
130_hand_back.png
210_upper_arm_front.png
220_forearm_front.png
230_hand_front.png
310_thigh_back.png
320_shin_back.png
330_foot_back.png
410_thigh_front.png
420_shin_front.png
430_foot_front.png
900_weapon.png
```

PNG 层约束：

- 必须为透明 PNG。
- 推荐全画布尺寸与 side 母版一致。
- 如果支持局部裁切 PNG，manifest 必须提供 `offsetPx`。
- 每层必须非空。
- 允许重叠，但必须有明确 `drawOrder`。

## 5. Rig Profile

### 5.1 `standard-side-humanoid-v1`

标准 17 层：

```text
hair_back
body
head
hair_front
upper_arm_back
forearm_back
hand_back
upper_arm_front
forearm_front
hand_front
thigh_back
shin_back
foot_back
thigh_front
shin_front
foot_front
weapon
```

用途：

- 作为正式 Spine 生产候选的推荐 profile。
- 可生成更可信的默认骨骼、slot、pivot 和 weapon socket。

### 5.2 `coarse-side-humanoid-v1`

粗粒度 7 层：

```text
body
head
arm_back
arm_front
leg_back
leg_front
weapon
```

用途：

- 快速验证流程。
- 新用户低门槛测试。
- 输出需标记为 `COARSE_RIG`，不得伪装为标准可生产质量。

### 5.3 `single-piece-reference`

单层参考模式：

```text
body
```

用途：

- 仅用于母版、身份、提示词、演示项目。
- 不允许进入正式动画导出门。

## 6. Manifest 配置规范

### 6.1 顶层结构

```json
{
  "schemaVersion": "f2s.layerpack/1",
  "packId": "black_hair_fighter__f2s_layerpack__v001",
  "displayName": "黑发格斗系测试角色",
  "characterSlug": "black_hair_fighter",
  "rigProfile": "standard-side-humanoid-v1",
  "canvas": {
    "width": 2048,
    "height": 2048,
    "unit": "px"
  },
  "views": [],
  "sources": [],
  "layers": [],
  "drawOrder": [],
  "sockets": [],
  "qaPolicy": {
    "allowOverlap": true,
    "compositeDiffMaxPercent": 2.0,
    "requireAllStandardRoles": true
  },
  "provenance": {
    "createdBy": "user-local-tool",
    "networkCallsMade": 0,
    "notes": ""
  }
}
```

### 6.2 View 配置

```json
{
  "viewKey": "side",
  "path": "views/black_hair_fighter__view_side__v001.png",
  "purpose": "primary-reference",
  "required": true
}
```

### 6.3 Layer 配置

```json
{
  "layerId": "layer.weapon",
  "role": "weapon",
  "path": "layers/standard/900_weapon.png",
  "displayName": "短木棍",
  "zIndex": 900,
  "offsetPx": { "x": 0, "y": 0 },
  "pivotPx": { "x": 1420, "y": 1030 },
  "boneHint": "hand_front",
  "required": true,
  "allowOverlapWith": ["hand_front", "body"],
  "notes": "右手自然下垂握持，木棍竖直贴身体外侧"
}
```

### 6.4 Socket 配置

```json
{
  "socketId": "primary-weapon",
  "semantic": "weapon-grip",
  "targetLayerId": "layer.weapon",
  "boneHint": "hand_front",
  "positionPx": { "x": 1420, "y": 1030 }
}
```

## 7. 新 QA 规则

### 7.1 废弃默认硬约束

旧规则：

```text
所有可见像素必须刚好属于一个层；任何重叠都是错误。
```

新规则：

```text
允许层重叠；以最终合成、层语义、透明边界、pivot/socket 和 draw order 判断是否可进入 Rig。
```

### 7.2 必需 QA

| QA | 规则 | 阻断级别 |
| --- | --- | --- |
| 文件结构 | manifest 存在，路径不越界，未知文件可列为 warning | BLOCKED |
| 图像格式 | layer 必须 PNG + Alpha | BLOCKED |
| 画布 | 全画布 PNG 必须尺寸一致；局部 PNG 必须有 offset | BLOCKED |
| 非空层 | required layer 必须有可见像素 | BLOCKED |
| 角色 profile | standard profile 必须覆盖 17 个标准 role | BLOCKED |
| draw order | 每个可见层必须有唯一 zIndex | BLOCKED |
| pivot | 参与 Rig 的层必须有 pivot，缺失可自动建议但需用户确认 | BLOCKED |
| socket | 主武器必须有 `weapon-grip` socket | BLOCKED |
| 合成对照 | composite 与 side reference 差异超阈值时阻断或需人工 waiver | BLOCKED / REVIEW |
| 重叠 | 允许，但必须可解释；未知大面积重叠给 warning | REVIEW |

### 7.3 合成差异

当存在 `preview/<slug>__composite_side__v001.png` 或 `views/side` 时，工具应计算：

- Alpha 覆盖差异。
- RGB 差异。
- 边缘 halo / 白边。
- 层组合后的 bounding box。

合成差异不是要求像素完全一致，而是用于发现：

- 图层漏导。
- 偏移错误。
- 画布尺寸不一致。
- 图层顺序明显错误。
- 半透明边缘被破坏。

## 8. 无 Photoshop 用户的产出路径

### 8.1 外部免费工具路径

用户可以使用：

- Krita：开源绘画工具，适合手工分层和导出透明 PNG。
- GIMP：开源图像编辑器，适合透明 PNG 层编辑。
- Clip Studio Paint：常见商业绘图软件，可导出 PSD。
- Photopea：在线 PSD 编辑器；若涉及隐私或商业资产，不推荐默认使用。

工具内只应提供导入规范和校验，不应声称这些工具已经通过项目发布许可审计。若要打包任何第三方组件，必须走 `F2S-DOC-SEC-001` 的许可和供应链审查。

### 8.2 内置 LayerPack Studio

可以做一个轻量内置工作区替代 PS 的一部分能力，但范围必须克制：

```text
导入 side reference
创建/重命名标准层
导入透明 PNG 到指定层
移动/缩放/旋转单层
编辑 pivot/socket
调整 draw order
查看合成预览
导出 LayerPack 文件夹
```

可选增强：

```text
套索/多边形选择
魔棒选区
边缘羽化
局部擦除
SAM 点选式 mask 候选
See-through / anime-segmentation 本地模型候选导入
```

不做：

```text
完整画笔系统
复杂调色
完整 PSD 编辑
在线模型调用
自动保证可商用素材
```

### 8.3 Codex Skill 与产品功能的边界

可以提供一个开发者/高级用户用的 Codex skill，例如 `f2s-layerpack-author`，用于：

- 生成资源包模板。
- 校验文件命名。
- 从 PSD 或 PNG 文件夹生成 manifest 草稿。
- 批量重命名。
- 输出导入问题报告。

但这个 skill 不能被视为面向终端用户的 PS 替代品。正式产品里应实现的是 `LayerPack Studio` 或 `LayerPack Import Wizard`，skill 只作为开发辅助和高级自动化入口。

### 8.4 其他工具是否可以内置

原则：

- 可以内置或调用，但必须逐项审计许可证、体积、运行环境、安全边界和模型权重来源。
- 宽松许可证也不等于可以无审计捆绑；模型权重、训练数据和商业使用条款需要单独记录。
- 未审计工具只能作为“用户自行安装的外部工具”，产品可导入其输出，不应捆绑。

候选类别：

| 类别 | 可用方式 | 风险 |
| --- | --- | --- |
| PSD 解析库 | 内置读取 PSD 层或作为构建工具 | PSD 特性复杂，需兼容测试 |
| Krita/GIMP | 用户外部编辑，导出 PNG | 不应把完整编辑器塞进主程序 |
| ImageMagick / libvips | 可用于批量转换和检查 | 需许可、CVE、分发体积审计 |
| SAM / anime segmentation | 本地可选模型候选 | GPU/CPU 性能、模型许可、输出不稳定 |
| See-through 类分层模型 | 本地候选 PSD/层 | VRAM 要求高，语义层不一定匹配标准 profile |

## 9. 架构设计模式

### 9.1 Strategy：输入策略

定义输入策略接口：

```text
LayerPackInputStrategy
  - PsdInputStrategy
  - TransparentPngFolderStrategy
  - GeneratedCandidateStrategy
```

每种策略都输出统一的 `LayerPackDraft`，后续校验、CAS、审批不关心来源。

### 9.2 Adapter：外部文件格式适配

```text
PsdReaderPort
ImageProbePort
LayerPackFsPort
LocalSegmentationPort
```

PSD、PNG、AI 候选都在 Adapter 层处理。Domain 只接收结构化 DTO 和 CAS hash。

### 9.3 Builder：LayerPack 构建器

`LayerPackBuilder` 负责：

- 标准化路径。
- 规范化 role。
- 生成默认 draw order。
- 生成 pivot/socket 草案。
- 收集 provenance。

### 9.4 Validator / Policy：可审批校验

`LayerPackPolicy` 拥有：

- role 完整性。
- 画布一致性。
- Alpha / 非空层检查。
- 重叠策略。
- 合成差异阈值。
- 主武器 socket 检查。

### 9.5 State Machine：导入向导

状态：

```text
EMPTY
SELECTED_FOLDER
PREFLIGHTED
MAPPED
QA_PASSED
PENDING_APPROVAL
APPROVED
REJECTED
```

任何文件变化、manifest 变化、映射变化或 pivot/socket 变化都应使审批失效。

## 10. 分层资源包导入流程

### 10.1 用户流程

1. 用户进入“分层与素材”。
2. 选择“导入分层资源包”。
3. 原生文件夹选择器选择 `<slug>__f2s_layerpack__vNNN`。
4. Native 读取 manifest 和文件列表。
5. 执行预检：路径、格式、尺寸、Alpha、非空层。
6. UI 显示映射表：文件 -> role -> draw order -> pivot -> bone hint。
7. 用户修正映射、pivot、weapon socket。
8. UI 显示合成预览和 side reference 对照。
9. QA 通过后创建 LayerSet 候选。
10. 用户原生确认并批准 Layer Gate。

### 10.2 与当前手绘工作台关系

- 当前“创建标准分层清单 + 手绘遮罩”保留为 `ManualMaskRepairWorkspace`。
- 新入口应在 UI 中位于手绘入口之前。
- 若用户导入 LayerPack，默认不要求手绘。
- 手绘修改只能作为 LayerPack 之后的修补 revision。

## 11. 代码改造计划

### M0：规范冻结与夹具

- 新增 `schemas/layerpack.schema.json`。
- 新增 `fixtures/layerpacks/valid-standard-pack`。
- 新增 `fixtures/layerpacks/valid-coarse-pack`。
- 新增 `fixtures/layerpacks/invalid-*` 覆盖路径越界、缺 Alpha、空层、缺 weapon socket。

验收：

- schema 校验可离线运行。
- 夹具不包含版权素材，使用程序生成的几何 PNG。

### M1：Domain / Application 模型

- 新增 Domain 实体：`LayerPackManifest`、`LayerPackLayer`、`ReferenceView`、`LayerPackQaReport`。
- 新增 use case：`preflight_layer_pack`、`create_layer_set_from_pack`。
- 保留现有 `LayerSet`，但新增 `overlap_policy` 或在 LayerPack -> LayerSet 的转换中记录 QA 依据。

验收：

- Domain 不依赖文件系统。
- 单元测试覆盖标准 / 粗粒度 / 缺失角色。

### M2：Native 文件夹导入 Adapter

- 使用 Windows 原生文件夹选择器。
- 读取 manifest 和相对路径。
- 防止路径穿越、符号链接越界和私有数据根污染。
- PNG 预检、Alpha 读取、尺寸检查、SHA-256 入 CAS。

验收：

- 不把原图或层图传给 WebView 解码。
- WebView 只拿受限预览和投影 DTO。

### M3：UI 导入向导

- 新增 `LayerPackImportWorkspace`。
- 显示资源包结构、视图、层列表、映射、draw order、pivot/socket。
- 显示合成预览和 QA 结果。
- 提供“创建 LayerSet 候选”和“原生确认并批准 Layer Gate”。

验收：

- 新用户不需要使用手绘笔刷即可完成分层审批。
- QA 阻断信息可读。

### M4：PSD 导入

- 先实现 PSD 作为用户外部文件输入，不把 PSD 编辑器内置进产品。
- 解析 PSD 图层名、可见性、层像素、offset。
- 自动映射明显命名：`head`、`body`、`weapon` 等。
- 模糊命名进入 UI 映射表，由用户确认。

验收：

- PSD 解析失败能降级提示“请导出透明 PNG 包”，不能损坏项目。
- PSD 图层哈希、导入时间、源文件名进入 provenance。

### M5：粗 Rig 自动生成

- 从 LayerPack layer + pivot + boneHint 创建 Rig 候选。
- standard profile 生成 17 层 Rig。
- coarse profile 生成粗 Rig 并在 UI 和导出 manifest 标记 `COARSE_RIG`。

验收：

- weapon socket 绑定 StyleSpec 的 `weapon-grip`。
- Rig 审批仍需人工确认。

### M6：LayerPack Studio

- 内置轻量工具，支持无 PS 用户生成资源包。
- 第一版只做导入、摆放、pivot/socket、draw order、导出，不做复杂绘画。
- 后续再接入可选本地分割模型。

验收：

- 可以从若干局部透明 PNG 拼成完整 LayerPack。
- 可以导出符合 schema 的文件夹。

### M7：本地 AI 候选 Spike

- 评估 See-through / SAM / anime-segmentation 作为本地候选生成器。
- 只产出候选 LayerPack，不自动批准。
- 记录硬件、模型权重、许可证、网络调用数、失败模式。

验收：

- 默认不开启。
- 没有模型时产品仍可正常工作。
- 真实能力不得标记为 PASS，除非有本地执行证据。

## 12. 测试矩阵

| 层级 | 测试 |
| --- | --- |
| Schema | manifest 正反例 |
| Domain | role 完整性、draw order、socket、policy |
| Adapter | 文件夹选择、路径越界、PNG Alpha、CAS hash |
| UI | 导入向导、映射编辑、QA 阻断、审批失效 |
| Integration | LayerPack -> LayerSet -> Rig -> MotionContent |
| Export | PSD/PNG/Spine JSON 输出仍绑定当前审批 |
| Security | 不读取私有根外越权路径，不上传图片，不保存绝对路径到 WebView |

## 13. 风险与未决

| 风险 | 影响 | 处理 |
| --- | --- | --- |
| PSD 格式复杂 | 图层效果、组、剪贴蒙版可能解析不完整 | V1 只支持普通像素层；复杂效果要求用户栅格化 |
| 允许重叠后 QA 更复杂 | 导出效果可能与预期不一致 | draw order + 合成预览 + 人工审批 |
| 无 PS 用户仍需要拆图能力 | 入口门槛仍高 | LayerPack Studio + 外部免费工具指引 |
| AI 分层不稳定 | 错层、缺层、伪影 | 只作为候选，不能自动过门 |
| 版权角色测试 | 商业发布风险 | 文档提示用户仅使用合法资产；项目不内置角色素材 |
| 第三方工具捆绑 | 许可证和供应链风险 | 默认外部工具；内置前走许可审计 |

## 14. 推荐落地顺序

优先级从高到低：

1. `transparent PNG folder` 导入：最小实现，立即绕开手绘遮罩痛点。
2. LayerPack schema + manifest：把用户文件夹变成可验证合同。
3. UI 映射表和合成预览：降低命名不匹配风险。
4. PSD 普通层读取：贴近美术生产习惯。
5. coarse profile：让新用户能快速跑通测试。
6. LayerPack Studio：服务没有 PS 的用户。
7. 本地 AI 分层候选：提高效率，但不作为质量承诺。

## 15. 用户文档草案摘要

用户应被明确告知：

```text
FlashToSpine 不要求你在程序里手绘拆图。
推荐你准备一个分层资源包：一个文件夹、一个 manifest、若干透明 PNG。
如果你会 PS/Krita/GIMP/Clip Studio，可以先在这些工具里分层。
如果你没有这些工具，可以使用内置 LayerPack Studio 摆放透明 PNG，或使用本地智能分割候选。
工具会检查命名、透明度、画布、层级、合成预览和武器 socket，然后再允许审批。
```

## 16. 总结

新的输入设计把“分层”从脆弱的手绘遮罩操作，改成可管理的资源包合同。PSD 和透明 PNG 包成为主路径，三视图只做参考，side 分层图进入 Rig；重叠不再天然错误，而是由 draw order、合成预览、pivot/socket 和人工审批共同管理。

这条路线更符合实际美术生产，也更适合后续接入本地 AI 候选、粗 Rig 快速测试和标准 Spine 生产候选。
