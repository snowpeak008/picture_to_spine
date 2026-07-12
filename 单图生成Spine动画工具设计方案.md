# 单图生成 Spine 动画工具设计方案

> 调研日期：2026-07-10  
> 当前目录状态：空项目，本文件用于立项和技术选型。  
> 目标假设：输入主要是 Image2 生成的单角色、二次元/卡通、全身图；首版输出目标锁定 Spine 4.2。

## 1. 结论先行

这件事可行，但产品形态不应承诺“任意一张图全自动得到可直接上线的高质量 Spine 动画”，更合理的目标是：

**产品目标可以设为 AI 自动完成 70%～90% 的分层、遮挡补全、骨架、网格和基础动画，再用一个轻量编辑器让用户修正低置信度结果。** 这个比例是待 Spike 验证的目标，不是现有项目已经证明的成功率。

当前最接近目标的开源组合是：

1. [See-through](https://github.com/shitagaki-lab/see-through)：把一张动漫角色图拆成最多 23 个语义明确、包含遮挡区域补全的 RGBA 图层，并输出 PSD、深度图和蒙版。
2. [Stretchy Studio](https://github.com/MangoLion/stretchystudio)：读取 See-through PSD，使用 DWPose/启发式方式自动绑骨，支持网格变形、时间轴，并已有 Spine JSON 导出原型。
3. [Spine 官方 JSON 格式](https://esotericsoftware.com/spine-json-format)和[导入接口](https://en.esotericsoftware.com/spine-import)：允许第三方工具生成 Spine 可导入的数据。

推荐方案不是简单地把两个仓库拼起来发布，而是：

- 复用 See-through 作为首版的动漫角色拆层后端；
- 参考或 fork Stretchy Studio 的 PSD 导入、编辑器和网格交互；
- 建立自己的、与 Spine 解耦的 Rig 中间格式；
- 重写一个版本锁定、可测试的 Spine 4.2 JSON 导出器；
- 通过用户本地、已授权的 Spine CLI 生成 `.spine`、`.skel` 和 atlas。

一个必须先说清的事实是：**单张图片只能主要决定“角色长什么样、初始姿势是什么”，不能决定“角色要做什么动作”。** 工具还需要一个动作来源：

- 首版：内置 `idle`、`breathe`、`blink`、`wave` 等动作模板；
- 第二阶段：参考视频或 BVH 动作重定向；
- 后续：文本生成动作，但不建议放进 MVP。

## 2. 为什么推荐“分层 2D Puppet”，而不是直接生成 AI 视频

| 路线 | 可编辑 Spine 骨架 | 角色一致性 | 遮挡处理 | 工程成本 | 建议 |
| --- | --- | --- | --- | --- | --- |
| 图生视频后逐帧导入 | 否，本质是帧动画 | 容易漂移 | 由视频模型隐式处理 | 低 | 不作为主线 |
| 单张纹理 + 一个 ARAP 网格 | 可以做基础骨骼 | 高 | 手臂离开身体时会露空或拉花 | 低 | 简化模式/兜底 |
| 语义拆层 + 补全 + 骨骼/网格 | 是 | 高 | 可显式控制 | 中 | **推荐** |
| 单图转 3D、再投影到 2D | 间接 | 中 | 由 3D 处理 | 很高 | 暂不做 |

AI 视频适合用来展示效果，或者作为动作参考；它通常不会提供 Spine 所需的稳定骨骼拓扑、槽位、网格、权重和可复用关键帧。把它作为主链路，会得到“能看但不能编辑”的结果。

## 3. 推荐的端到端流程

```text
Image2 生成的 PNG/JPEG
        │
        ▼
输入预检：单人、全身、未裁切、四肢可辨、背景质量
        │
        ▼
角色/背景分离 + 语义部件拆层 + 遮挡区域补全 + 初始深度顺序
        │
        ▼
人工复核：蒙版、左右部件、前后顺序、补全像素
        │
        ▼
关节检测 → 标准骨架模板 → pivot/父子关系 → 置信度修正
        │
        ▼
轮廓网格 → 三角剖分 → 骨骼权重 → 关节弯曲测试
        │
        ├──────── 动作预设 / 参考视频 / BVH
        ▼
动画重定向、平滑、足底锁定、关键帧精简、次级运动
        │
        ▼
内部 Rig IR（唯一事实来源）
        │
        ▼
Spine 4.2 JSON + PNG → 格式/视觉验证
        │
        ▼
可选：用户本地 Spine CLI → .spine / .skel / .atlas / atlas PNG
```

### 3.1 输入预检与上游生成约束

如果能控制 Image2 的生成提示词，输入质量比更换后处理模型更重要。推荐默认约束：

```text
单个完整角色，全身，正面或轻微 3/4 视角，中性 A-pose；
双臂与躯干留出间隙，双腿分开，双手双脚完整可见；
不拿道具，没有前景遮挡，不裁切头发或脚；
纯色或透明背景，干净清晰的轮廓，均匀光照。
```

工具应给输入计算一个可动画性分数，并明确提示问题，例如：

- 身体被画面边缘裁切；
- 左右手臂互相遮挡或贴住躯干；
- 长裙把双腿全部遮住；
- 背景与轮廓颜色接近；
- 多角色、非人形或极端透视；
- 图片只有胸像，却选择了全身动作。

最好同时提供两个入口：

- `Animation-ready`：按照上述约束生成/上传图片，自动化率最高；
- `Best effort`：接受任意图片，但展示更多人工修正步骤。

### 3.2 图层拆分与遮挡补全

动漫/VTuber 风格首选 See-through。它使用透明图层生成、动漫深度估计和语义分割，把头发、脸、眼睛、服装、鞋、饰品等拆到独立图层。官方说明默认 1280 分辨率约需 12～16 GB 显存，也提供约 8 GB 峰值的 NF4 量化路径。它的局限是训练域偏动漫风格，真实人物、厚涂、像素风和非人角色可能失败。

非动漫图片可考虑两级兜底：

1. [Qwen-Image-Layered](https://huggingface.co/Qwen/Qwen-Image-Layered) 先做通用 RGBA 图层分解；
2. [BiRefNet](https://github.com/ZhengPeng7/BiRefNet) 做高质量前景 alpha，再配合 SAM 类交互分割和局部补全。

Qwen-Image-Layered 能输出可递归拆分的通用图层，但这些图层不一定对应“上臂、前臂、躯干”等可绑骨语义，所以通用模式仍需语义标注或用户确认。它也不是轻量本地模型：官方仓库文件很大，[已有 16 GB GPU OOM 报告](https://github.com/QwenLM/Qwen-Image-Layered/issues/8)。更适合对高价值失败部件做服务器 fallback，而不是默认处理每张图。

如果 See-through 的画风或权重许可不符合项目要求，可建立一条许可证相对易审计、但人工量更大的组合：

1. [BiRefNet](https://github.com/ZhengPeng7/BiRefNet) 提取角色外轮廓；
2. [Grounding DINO](https://github.com/IDEA-Research/GroundingDINO) + [SAM 2](https://github.com/facebookresearch/sam2) 做文本引导和交互部件分割；
3. [ViTMatte](https://github.com/hustvl/ViTMatte) 细化头发、薄纱等 alpha 边缘；
4. [Depth Anything V2 Small](https://github.com/DepthAnything/Depth-Anything-V2) 提供弱深度先验；
5. [LaMa](https://github.com/advimman/lama) 补全较简单的衣服、皮肤和背景区域。

这条链路中的通用模型不理解动漫生产语义，必须增加部件映射、蒙版笔刷、左右拆分和补全重跑 UI。还要逐个核对权重许可证；例如 Depth Anything V2 的 Small 与其他尺寸权重并非同一许可，不能只看仓库代码许可证。

实现时应遵守一个重要原则：

> 原图中可见的像素尽量直接复制，只有原图中被遮挡的区域才交给生成模型补全。

这样可以最大程度保持 Image2 原图的角色一致性。建议为每个像素保存 `visible/original` 与 `hallucinated/inpainted` 标记，并在编辑器中允许高亮所有 AI 猜测区域。

初始绘制顺序可以由深度图和语义规则共同决定，但不要简单地按“整层平均深度”排序。更稳的做法是建立成对遮挡图：在图层交叠边界判断 `A 在 B 前`，把深度中位数作为弱先验，再对约束图做拓扑排序。如果出现环，说明一个部件在不同区域同时穿到另一个部件前后，必须把该层继续切开。动作期间也可能需要改变顺序；例如挥手越过脸部时，应生成 Spine `draworder` 时间线，不能只依赖一个静态 Z 值。

### 3.3 自动骨架

首版限定“单个类人角色”会大幅降低难度。推荐骨架模板：

```text
root
└─ pelvis
   ├─ torso ─ chest ─ neck ─ head
   │                 ├─ upper_arm_l ─ lower_arm_l ─ hand_l
   │                 └─ upper_arm_r ─ lower_arm_r ─ hand_r
   ├─ thigh_l ─ shin_l ─ foot_l
   └─ thigh_r ─ shin_r ─ foot_r
```

头发、裙摆、尾巴、翅膀和饰品作为可选附加骨链。

关节初值可使用 [DWPose](https://github.com/IDEA-Research/DWPose) 或 [MMPose/RTMPose](https://github.com/open-mmlab/mmpose)；随后结合部件蒙版、轮廓中轴和左右语义标签修正。DWPose 是真人全身姿态模型，面对 Q 版比例、夸张服装或非人角色时必须允许用户拖动关节点。

每个预测关节都应有置信度。低于阈值时不要静默生成错误骨架，而应在“骨架检查”步骤中逐个高亮。关节 pivot 应放在肩、肘、腕、髋、膝、踝附近，而不是简单使用图层中心。

### 3.4 网格和权重

建议的自动网格流程：

1. 从 alpha 蒙版提取轮廓并简化；
2. 在关节两侧和高曲率轮廓处增加支撑点；
3. 使用约束 Delaunay 三角剖分生成网格；
4. 刚性部件默认绑定一个骨骼；
5. 躯干、四肢连接处、头发和布料用热扩散/调和权重；
6. 每顶点保留少量主要影响骨骼，归一化并裁掉极小权重；
7. 自动运行肩、肘、膝的弯曲测试并检测裂缝、拉花和反三角形。

[Spine 官方网格说明](https://eu.esotericsoftware.com/spine-meshes)也建议控制顶点数和每顶点的骨骼影响数，因为这些都会增加运行时 CPU 计算。首版不要追求很密的网格；“部件拆得合理 + 少量正确顶点”通常比“单层超密网格”效果更稳。

### 3.5 动画来源

建议按以下顺序实现：

#### A. 参数化动作预设（MVP）

- `idle`：骨盆/胸腔轻微上下移动；
- `breathe`：胸腔缩放、肩部微动；
- `blink`：眼睑附件切换或眼部网格变形；
- `wave`：肩、肘、腕的标准角度曲线；
- 可选 `hair_sway`：头发附加骨链的相位滞后。

动作以“归一化骨骼角度和角色身高比例”表达，而不是写死像素坐标，便于复用到不同比例角色。

#### B. 参考视频（第二阶段）

对视频逐帧运行姿态估计，得到 2D 关节序列，再进行：

- 置信度过滤与缺帧插值；
- One-Euro/低通滤波去抖；
- 骨长约束与关节角限制；
- 足底锁定，减少脚滑；
- 映射到目标角色骨骼；
- 曲线拟合和关键帧精简。

#### C. BVH/动作库（第二阶段）

[AnimatedDrawings](https://github.com/facebookresearch/AnimatedDrawings) 已证明“单图自动检测/分割/绑骨 + BVH 重定向 + ARAP 变形”是可行的，代码和模型权重为 MIT。它已经在 2025 年归档，输出也不是 Spine，但其重定向、标注修正界面和 ARAP 实现很值得参考。

#### D. 文本生成动作（后续）

文本动作模型最终仍要落到稳定的骨骼曲线，并处理脚滑、碰撞和关键帧数量。对 MVP 来说，固定模板和参考视频更可控，也更容易验收。

### 3.6 表情与口型

单张图通常只有一个眼睛和嘴型。基础眨眼可以通过裁剪、缩放或网格变形近似；更自然的闭眼、微笑和口型需要额外附件。

后续可让图像编辑模型基于原图生成一组经用户确认的附件：

- 眼睛：open、half、closed；
- 嘴：neutral、smile、A/I/U/E/O；
- 手：open、fist；
- 可选武器/服装 skin。

这些附件适合使用 Spine slot attachment 切换，而不是把所有变化都做成骨骼缩放。

## 4. 推荐的软件架构

### 4.1 模块边界

| 模块 | 建议实现 | 责任 |
| --- | --- | --- |
| 编辑器前端 | React + WebGL/Canvas，优先参考 Stretchy Studio | 图层、蒙版、骨架、pivot、网格、权重和时间轴修正 |
| AI 推理服务 | Python + PyTorch/Diffusers，本地进程或局域网服务 | See-through、背景分离、姿态估计、补全 |
| Rig Core | TypeScript、Rust 或 Python 中的纯数据/数学模块 | 坐标、层级、网格、权重、动画重定向 |
| 项目存储 | ZIP + JSON manifest + PNG/mask/depth | 保存可重复编辑的全部中间结果 |
| Spine Exporter | 独立的版本化 serializer | 只把内部 IR 映射到 Spine 4.2 JSON |
| Validator | 静态检查 + Spine CLI/目标 Runtime 可选验证 | 格式、引用、坐标、视觉和动画回归 |

MVP 最省事的部署方式是“浏览器编辑器 + 本地 Python GPU 服务”。直接把 PyTorch/CUDA 模型打进 Tauri/Electron 安装包会显著增加安装、显卡驱动和升级复杂度，可以等流程稳定后再桌面化。

### 4.2 内部 Rig IR

不要直接在业务代码中到处读写 Spine JSON。先定义自己的中间格式，例如：

```json
{
  "canvas": { "width": 1280, "height": 1280, "yAxis": "down" },
  "layers": [
    {
      "id": "arm_l",
      "semantic": "left_arm",
      "image": "layers/arm_l.png",
      "mask": "masks/arm_l.png",
      "z": 12,
      "generatedRegionMask": "masks/arm_l_inpainted.png"
    }
  ],
  "bones": [],
  "attachments": [],
  "meshes": [],
  "animations": [],
  "provenance": { "models": [], "seeds": [], "sourceHash": "..." }
}
```

这样可以：

- 日后同时导出 Spine、Live2D、Inochi2D 或自有运行时；
- 在不重新跑 AI 的情况下修改导出版本；
- 保存模型版本、随机种子和人工修改；
- 为每个阶段建立可复现测试。

## 5. Spine 输出策略

### 5.1 首版输出包

```text
character-project.zip
├─ project.json                 # 自有 Rig IR
├─ character.json               # Spine 4.2 JSON
├─ images/*.png                 # 独立附件图片
├─ masks/*.png                  # 可选，供继续编辑
├─ preview/setup-pose.png
├─ preview/idle.gif
└─ validation-report.json
```

如用户本地装有已授权 Spine，再生成：

```text
character.spine
runtime/character.skel 或 character.json
runtime/character.atlas
runtime/character.png
```

### 5.2 为什么不直接写 `.spine`

`.spine` 是编辑器项目文件，官方没有提供第三方项目文件 writer 规范。官方支持的互操作路径是第三方生成 JSON，然后由 Spine GUI/CLI 导入。建议命令形态：

```powershell
# 把第三方 JSON 导入为可编辑项目；生产中固定准确的 4.2 patch 版本
Spine -u 4.2.XX -i character.json -o character.spine -r Character

# 使用从 Spine 导出窗口保存的 export-settings.json 生成运行时数据和 atlas
Spine -u 4.2.XX -i character.spine -o runtime -e export-settings.json
```

Spine CLI 属于 Spine Editor，不是免费的服务器转换服务。公共 SaaS 更稳妥的边界是服务端只生成 JSON/PNG，让用户本地 Spine 完成项目、二进制和 atlas 转换。

Spine 4.2 也支持在编辑器 UI 中[直接导入带标签的 PSD](https://en.esotericsoftware.com/spine-import-psd)，可作为人工工作流的备用出口；但公开 CLI 文档的无界面导入对象没有列出 PSD，而且 PSD 本身也不包含自动权重和动画，因此自动化主出口仍应是 JSON + PNG。

### 5.3 Spine 4.2 映射要点

- `bones` 中父骨必须先于子骨；
- 图像层映射为 `slots` 和 region/mesh attachments；
- 设置姿势坐标是父骨局部坐标，不是画布绝对坐标；
- 常见画布是 Y 向下，Spine 是 Y 向上，必须统一翻转规则；
- 裁切透明边后必须保留原画布 offset；
- 加权 mesh 的 `vertices` 是 Spine 特有的紧凑编码；
- 动画至少覆盖 bone rotate/translate/scale、slot attachment/color，以及需要时的 deform/draworder；
- 若还要导回编辑器继续修改，应保留必要且已验证的 nonessential data；错误的 mesh `edges` 等编辑数据反而可能令导入失败；
- Editor、JSON 和 Runtime 的 major/minor 必须一致，首版固定 4.2，不追 4.3 beta。

官方格式示例可直接使用 [spine-runtimes 4.2 examples](https://github.com/EsotericSoftware/spine-runtimes/tree/4.2/examples) 作为 golden fixtures。

## 6. 开源项目评估

| 项目 | 能解决什么 | 许可证/状态 | 采用建议 |
| --- | --- | --- | --- |
| [See-through](https://github.com/shitagaki-lab/see-through) | 单张动漫图到最多 23 个补全语义层、深度和 PSD | 仓库 Apache-2.0；2026 SIGGRAPH 条件接收 | **首选拆层后端**，但逐个审计模型权重 |
| [ComfyUI-See-through](https://github.com/jtydhr88/ComfyUI-See-through) | 更易部署的节点封装、PSD 下载、显存优化 | MIT；已有 8～24 GB 档位说明 | 适合快速 PoC 和基准测试 |
| [Stretchy Studio](https://github.com/MangoLion/stretchystudio) | See-through PSD、自动绑骨、网格、时间轴、Spine 导出 | MIT；项目很新，无正式 release | 参考/fork 编辑器，**不要原样依赖导出器** |
| [AnimatedDrawings](https://github.com/facebookresearch/AnimatedDrawings) | 单图检测、分割、关节、BVH 重定向、ARAP | MIT；2025-09 已归档 | 参考简化模式和动作重定向 |
| [Qwen-Image-Layered](https://github.com/QwenLM/Qwen-Image-Layered) | 通用图片拆成多个 RGBA 层，可递归拆分 | Apache-2.0；模型很大 | 服务器端高质量 fallback；仍需身体语义化 |
| [BiRefNet](https://github.com/ZhengPeng7/BiRefNet) | 高分辨率前景/背景 alpha | MIT | 输入预处理；不要误用仅限非商用的第三方 RMBG 权重 |
| [DWPose](https://github.com/IDEA-Research/DWPose) | 全身、脸、手部关键点 | Apache-2.0 | 自动骨架初值，必须可人工修正 |
| [SDPose-OOD](https://github.com/T-S-Liang/SDPose-OOD) | 面向 anime/sketch/遮挡等域外图的 17/133 点姿态 | MIT 标记；依赖链需另审计 | DWPose 失败时的实验 fallback，不直接作为默认商用依赖 |
| [MMPose](https://github.com/open-mmlab/mmpose) | 多种人/动物姿态模型和统一推理接口 | Apache-2.0 | DWPose 替代或未来非人扩展 |
| [Inochi Creator](https://github.com/Inochi2D/inochi-creator) | 开源 2D puppet 编辑器 | 开源项目 | 参考网格、参数和编辑器交互，不直接解决 Spine 输出 |
| [spine-scripts](https://github.com/EsotericSoftware/spine-scripts) | 官方图层工具到 Spine 的脚本 | 各目录分别检查许可证 | 参考坐标、图层裁切、slot/skin/命名 |
| [Spine-IO](https://github.com/SimonHeggie/Spine-IO) | Blender 骨架、mesh、UV、权重、FK 到 Spine JSON | GPL-3.0、alpha、目标偏 4.3 | 研究格式映射；闭源产品不要直接复制 GPL 代码 |
| [N-Sprite](https://github.com/lucaspedrajas/N-Sprite) | VLM 规划零件、分割、补全、父子层级、pivot 和 atlas | 极早期；仓库暂无正式 LICENSE；依赖付费 API | 借鉴流水线思路，不直接集成 |

### 6.1 对 Stretchy Studio 的具体判断

它是目前最接近本需求的现成项目，README 明确写有 See-through、DWPose 自动绑骨和 Spine 4.0 导出，因此非常适合做一周内的可行性验证。

但截至调研时：

- 仓库没有正式 release，仍是快速迭代期；
- [已有 issue 报告导出 JSON 出现非法数字](https://github.com/MangoLion/stretchystudio/issues/4)；
- 当前 [`exportSpine.js`](https://github.com/MangoLion/stretchystudio/blob/master/src/io/exportSpine.js) 主要覆盖基础 bone transform、slot opacity、region/mesh 和图片打包；没有形成完整的 IK、权重、deform、draw order 等生产级导出闭环；
- 导出头部硬编码为 4.0，而本方案建议目标为稳定的 4.2。

因此应把它定位为“优秀的产品原型和代码参考”，不是未经验证即可上线的 SDK。

### 6.2 See-through 的许可证注意点

See-through 仓库代码是 Apache-2.0，主要 LayerDiff 模型页面也标注 Apache-2.0。但其部分深度/身体解析 checkpoint 的模型页缺少完整 model card 或明确 license。商业使用前必须把以下三件事分开审计：

1. 代码许可证；
2. 每一个模型权重的许可证；
3. 训练数据及模型输出的使用边界。

在完成确认前，可以把 See-through 作为 R&D 基线；生产版可替换许可证不清晰的深度/解析模型，或向作者取得书面确认。

## 7. MVP 建议

### 阶段 0：5 天左右的技术 Spike

不写完整产品，先验证最危险的假设：

1. 准备 20～30 张测试图：标准动漫、Q 版、长裙、复杂头发、厚涂、非动漫；
2. 跑 See-through/ComfyUI-See-through，统计时间、显存、图层正确率和补全质量；
3. 把 PSD 放进 Stretchy Studio，测试自动骨架、网格和 4 个基础动作；
4. 导出 JSON，并用 Spine 4.2 导入；
5. 记录每张图的人工修正分钟数和失败原因。

Spike 的退出条件不是“最好看的样例成功”，而是能够回答：

- 哪类输入可自动完成；
- 平均需要修几处、几分钟；
- JSON 是否能稳定进入 Spine 4.2；
- 最低可接受 GPU 配置；
- 哪些权重可以用于目标发布方式。

### 阶段 1：可交付 MVP

- PNG/JPEG 输入和预检；
- See-through 拆层；
- 图层、蒙版、左右/前后顺序修正；
- DWPose 自动关节和拖拽修正；
- 基础 region 或低密度 mesh；
- `idle / breathe / blink / wave`；
- 自有 Rig IR；
- Spine 4.2 JSON + PNG 导出；
- 静态检查、原姿势复原图和 Spine CLI 可选验证。

### 阶段 2：生产质量

- 自动权重和关节弯曲 QA；
- 参考视频/BVH 动作重定向；
- draw order 动画、IK、附件切换；
- 头发/布料次级运动；
- 表情与口型附件；
- atlas、批处理、断点恢复和模型缓存；
- 多个目标 Runtime 的回归测试。

### 阶段 3：扩展

- 非人角色骨架模板；
- 通用画风分层模型；
- 多视图或多姿势输入；
- 文本生成动作；
- Live2D/Inochi2D 等其他导出器。

## 8. 验收与测试

### 8.1 格式验收

- JSON 是合法 JSON，所有名称、引用、顶点索引和权重有效；
- 父骨顺序、slot attachment、skin、动画 target 全部可解析；
- 固定 Spine 4.2 patch 的 CLI 导入无错误；
- 导入后重新导出，再由目标 Spine Runtime 加载成功；
- 不使用“某一个例子能打开”代替自动化回归。

### 8.2 视觉验收

- setup pose 与原图合成结果做像素差异图；
- 可见原图区域不得被生成模型无故改写；
- 肩、肘、髋、膝弯曲时无明显裂缝、透明洞和反转三角形；
- `idle` 和 `breathe` 首尾帧无跳变；
- 前后遮挡和 slot draw order 符合动作；
- 对每张测试图记录人工修正时间，而不只记录推理时间。

### 8.3 建议的产品指标

- 自动拆层后无需重画的样本比例；
- 关节点一次预测可接受比例；
- 每角色人工修正中位时间；
- Spine 导入一次成功率；
- 四个默认动画可直接使用比例；
- 8 GB、12 GB、16 GB、24 GB 显卡上的峰值显存和耗时。

## 9. 主要风险和对应策略

| 风险 | 表现 | 策略 |
| --- | --- | --- |
| 单图信息不足 | 被遮挡肢体、闭眼、口腔没有真实像素 | AI 补全 + 标记生成区域 + 用户确认 |
| 风格域偏差 | 真实图、厚涂、怪物拆层失败 | MVP 限定动漫类人；通用模式作为实验功能 |
| 关节误检 | Q 版、长裙、极端姿势骨架错误 | 置信度门控 + 可拖拽关节 + 模板约束 |
| 运动露馅 | 关节处开洞、贴图拉花 | 分层补全、关节支撑点、弯曲自动测试 |
| 深度顺序错误 | 手臂/头发穿插关系不对 | 深度 + 语义规则 + draworder 时间线 + 人工修正 |
| Spine 版本变化 | JSON 可打开但 Runtime 不兼容 | 目标锁定 4.2；版本化 exporter 和 golden fixtures |
| 开源项目过新 | API、输出或模型快速变化 | pin commit/hash；自有 IR；不要把 exporter 绑死到第三方 |
| 商业许可不清晰 | 代码许可和权重许可不一致 | 逐依赖清单审计，保存 NOTICE，必要时书面确认/替换 |

## 10. Spine 与商业授权边界

以下是技术调研结论，不构成法律意见。

- Spine 官方文档明确支持从其他程序导入同格式 JSON；[官方人员也确认](https://en.esotericsoftware.com/forum/d/29331-json-file-coming-in-with-invisible-geometry/12)，未使用 Spine Runtime 的第三方脚本可以生成和分享 Spine 可读数据。
- 若产品嵌入 `spine-ts`、`spine-cpp`、`spine-unity` 等官方 Runtime，需要遵守 [Spine Runtimes License](https://en.esotericsoftware.com/spine-runtimes-license)；建议 MVP 自己预览 Rig IR，不嵌入官方 Runtime。
- Spine CLI 是付费编辑器的一部分。不要在公共 SaaS 上把一份普通个人许可证作为多人共享转换服务；如需服务端集群，应向 Esoteric Software 确认 Enterprise 方案。
- 包含 mesh、weights、IK、clipping 等功能的项目通常需要 Spine Professional；[Spine Essential 无法保存或导出包含这些 Professional 功能的项目](https://us.esotericsoftware.com/spine-purchase)。
- 用户还必须拥有输入 AI 图片、训练/参考素材和最终角色资产的合法使用权。

## 11. 最终推荐决策

如果现在就立项，我会选择以下边界：

1. **只承诺动漫/卡通类人全身角色**，先把成功率做高；
2. **See-through 负责拆层，DWPose 负责关节初值**；
3. **参考 Stretchy Studio 做编辑体验，但自建 Rig IR 和 Spine 导出器**；
4. **首版只做四个参数化动作**，参考视频放到下一阶段；
5. **输出 Spine 4.2 JSON + PNG**，不直接逆向写 `.spine`；
6. **用户本地 Spine CLI 完成 `.spine/.skel/atlas`**；
7. **每一步都允许修正并显示置信度/AI 补全区域**；
8. **先做 20～30 张图的 Spike，再决定是否产品化**。

这个方案的优势是，最难的“单图拆成有遮挡补全的图层”已经有很接近目标的开源研究成果；真正需要投入的核心竞争力会集中在：稳定的人工修正体验、自动网格/权重、动作重定向、Spine 兼容测试，以及对失败样本的可解释处理。

## 12. 主要资料

### Spine 官方

- [JSON export format](https://esotericsoftware.com/spine-json-format)
- [Import data](https://en.esotericsoftware.com/spine-import)
- [Command line interface](https://esotericsoftware.com/spine-command-line-interface)
- [Versioning](https://esotericsoftware.com/spine-versioning)
- [Mesh attachments](https://eu.esotericsoftware.com/spine-meshes)
- [Weights](https://us.esotericsoftware.com/spine-weights)
- [Import PSD](https://en.esotericsoftware.com/spine-import-psd)
- [Texture atlas format](https://en.esotericsoftware.com/spine-atlas-format)
- [Spine Runtimes repository](https://github.com/EsotericSoftware/spine-runtimes)
- [Spine Runtimes License](https://en.esotericsoftware.com/spine-runtimes-license)

### 单图拆层、绑骨和动画

- [See-through repository](https://github.com/shitagaki-lab/see-through)
- [See-through paper](https://arxiv.org/abs/2602.03749)
- [See-through LayerDiff model](https://huggingface.co/layerdifforg/seethroughv0.0.2_layerdiff3d)
- [ComfyUI-See-through](https://github.com/jtydhr88/ComfyUI-See-through)
- [Stretchy Studio](https://github.com/MangoLion/stretchystudio)
- [AnimatedDrawings](https://github.com/facebookresearch/AnimatedDrawings)
- [Qwen-Image-Layered](https://huggingface.co/Qwen/Qwen-Image-Layered)
- [DWPose](https://github.com/IDEA-Research/DWPose)
- [MMPose](https://github.com/open-mmlab/mmpose)
- [BiRefNet](https://github.com/ZhengPeng7/BiRefNet)
- [Inochi Creator](https://github.com/Inochi2D/inochi-creator)

### 可参考的导出实现

- [Stretchy Studio exportSpine.js](https://github.com/MangoLion/stretchystudio/blob/master/src/io/exportSpine.js)
- [EsotericSoftware/spine-scripts](https://github.com/EsotericSoftware/spine-scripts)
- [SimonHeggie/Spine-IO](https://github.com/SimonHeggie/Spine-IO)
- [Spine 4.2 official examples](https://github.com/EsotericSoftware/spine-runtimes/tree/4.2/examples)
