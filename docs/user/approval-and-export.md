# 人工审批与导出说明

## 1. 审批不是“点过按钮”

FlashToSpine 的审批绑定当前对象 ID、revision、规范化内容 SHA-256、人工 actor 和确认时间。宿主先展示 Windows 原生确认框，再把一次性 attestation 交给领域服务；取消、重复使用、目标哈希变化或 revision 过期都会失败。

审批证明的是“本机交互用户明确接受了这一个候选”，不是自动质量保证，也不是法律意义上的个人签名。项目头的 HMAC 能发现本地文件篡改、回滚或分叉，但不能证明真实姓名、组织职务或不可否认性。

安全 store 的 authenticated roll-forward 也不会改变审批语义。它只在进程中断后，沿 HMAC 有效且直接延续当前 revision 的不可变 signed sidecar，把 anchor/`head.json` 投影恢复到已经提交的较新 revision；它不会创造审批、接受 unsigned 数据或把旧 revision 重新变成当前事实。

## 2. 六类人工检查

### 2.1 母版审批

检查角色身份、二次元风格、横版侧视轮廓、画布和唯一主武器。更换图片、StyleSpec 或武器语义后，旧审批不再适用于新候选。

### 2.2 分层审批

检查 LayerSet 的图层完整性、顺序、可见性、遮罩和重组结果。遮挡区域缺少真实像素时，工具不能从单图恢复事实；应先由美术人员补齐或修正。

### 2.3 Rig 审批

检查骨骼树及其 X/Y/rotation/Scale X/Scale Y rest transform、父关系、slot bone binding/draw key、pivot 和主武器 socket。mesh、weight 和约束只作为只读能力摘要检查，不能描述成完整编辑器。Rig 必须绑定当前 LayerSet 审批哈希以及固定能力 `F2S-SPINE-CAP-4.2.43-001`。

### 2.4 关键姿势图片审批

MotionContent 的素材 BOM 会列出每个动作所需的本地图片。每个 required AssetSpec 都必须绑定一张通过完整解码的图片，并经人工核对预览、动作/pose 用途、尺寸、Ground Y、Scale 和哈希。调整 Ground Y/Scale 会增加 binding/content revision，并使该素材回到待审核状态；PromptPack 只是本地生成的创作提示，不代表图片已经产生或合格。

### 2.5 十动作关键姿势审批

十个动作各有独立 clip 和 pose approval：`idle`、`run`、`jump`、`fall`、`dash`、`attack_01`、`attack_02`、`attack_03`、`hit`、`death`。任何轨道、关键帧或 marker 修改都会改变审批 payload，相关动作需要重审。

Pixi 工作预览只显示刚性单骨 attachment 和当前骨骼变换，不模拟 mesh 变形、多骨骼权重或完整 Spine Runtime。预览是人工检查输入，不是 Editor 等价渲染证据。

### 2.6 三个命中帧审批

仅 `attack_01`、`attack_02`、`attack_03` 需要 hit approval。每个攻击必须有且仅有一个单 tick 命中 marker，位于对应 MotionSpec 的 `contact` phase 内，并引用当前唯一主武器 socket。命中帧审批依赖当前 pose approval；改变命中 tick/socket 会使该动作的 Hit 审批失效，但保持已通过的 Pose 审批。

## 3. 失效传播

上游事实变化会让依赖它的批准失效。典型链路如下：

```text
母版 / StyleSpec / 主武器变化
  -> LayerSet、Rig、MotionContent、AnimationSet 与下游审批需要重算

LayerSet / 遮罩 / draw order 变化
  -> Rig 与动画相关审批需要重算

Rig / slot binding或drawKey / socket / 骨骼rest或父关系 / pivot变化
  -> AnimationSet、十个 pose 与三个 hit 审批需要重算

MotionSpec / 关键姿势素材绑定或对齐变化
  -> 只使对应动作的 pose，以及依赖 pose 的 hit 审批需要重算

命中 tick / socket 变化
  -> 只使对应攻击动作的 hit 审批需要重算；pose 审批保持有效

Clip / 轨道 / marker 变化
  -> 对应 pose；攻击动作还会使 hit 审批失效
```

界面显示 `APPROVED` 以前，导出器仍会从当前项目 manifest 重新计算规范化哈希和闭包。缓存状态、旧导出记录或手改 JSON 不能替代当前审批。

## 4. 导出预检必须同时满足什么

导出器只从当前权威 ProjectManifest 装配 PublishSnapshot，不接受 UI 自报的 READY。至少需要：

- 当前母版、LayerSet、Rig 都有未失效且哈希匹配的人工审批；
- 所有 required 关键姿势图片均有当前人工审批；
- MotionContent 与 AnimationSet 恰好包含固定十动作；
- 十个 pose approval 和三个攻击 hit approval 完整；
- StyleSpec、Rig 和主武器 socket 的单武器语义一致；
- 骨骼、slot、pivot、socket、mesh、约束和整数 tick 时间轴有效；
- Spine patch 精确为 4.2.43；
- attachment 路径限制在 `images/`，PNG 哈希和 Windows 文件名安全；
- 每个顶点当前只能刚性绑定一个骨骼，不能含多骨骼权重；
- 导出根目录及其规范化路径不位于整个 `%LOCALAPPDATA%\FlashToSpine` 私有数据根中；不是只排除 `projects`、`cas`、`staging` 三个已知子目录。

任一项失败都保持 `BLOCKED`。不允许通过降低检查、伪造审批或改状态字符串强行导出。

## 5. 开放格式包的所有权

| 文件 | 生成方 | 说明 |
| --- | --- | --- |
| `rig-ir.json` | FlashToSpine 内置 writer | 权威 Rig/clip/marker 交换快照 |
| `character.psd` | FlashToSpine 内置 writer | 最小分层 PSD；可人工修订，但不是 Spine 工程真值 |
| `images/layer-*.png` | FlashToSpine 内置 writer | 从已批准 attachment 物化的透明 PNG |
| `character.spine.json` | FlashToSpine 内置 writer | 固定 4.2.43 的 JSON candidate |
| `atlas-input-manifest.json` | FlashToSpine 内置 writer | atlas 输入路径、哈希和 packing 输入清单，不是 `.atlas` |
| `prompt-pack.json/.md` | FlashToSpine 内置 writer | 动作关键帧图片的开发提示词和上下文 |
| `compatibility-manifest.json` | FlashToSpine 内置 writer | 版本、能力和外部状态合同 |
| `checksums.sha256` | FlashToSpine 内置 writer | 包内文件完整性清单 |
| `.atlas` / `.spine` / `.skel` | 仅用户本地 Professional CLI/Editor | 专有格式；内置 writer 永不创建 |

输出先写入同一导出根下的 staging 子目录，完成哈希和清单复核后再改名提交。已有 export ID 或 staging ID 不会被覆盖。导出历史绑定源 project revision、PublishSnapshot SHA-256、输出 checksums 和外部状态。

如果文件包已提交、但随后项目导出历史写入失败，文件包仍保持不可变，并在 `%LOCALAPPDATA%\FlashToSpine\export-recovery` 留下恢复记录。此时先保留包和恢复记录，不要重复使用同一目录或删除证据。

## 6. Spine Professional CLI/Editor 边界

设置页、导出页和原生宿主已经接入三类异步 CLI job，但本产品只允许调用用户自己合法安装的规范 `Spine.com`。安全适配器要求：

- 可执行文件名和规范路径匹配，拒绝 reparse/symlink 路径；
- 探针和操作都精确报告 4.2.43；
- 固定参数形状，不经过 shell，不带更新或激活开关；
- 每次操作绑定可执行文件哈希、输入、输出目录和一次性人工确认；
- 有超时、输出大小和文件快照预算；
- 记录本轮 provenance，不读取或复制激活信息。

允许的外部操作类别是 JSON 导入为 `.spine`、从 `.spine` 导出 `.skel`，以及根据输入目录和 settings 生成 `.atlas`。这些文件写到开放包之外的新目录，不能回写或污染已提交的不可变开放包。

操作流程分为两层：

1. “设置”只登记用户原生选择的 `Spine.com`、可执行文件 SHA-256、不可逆 path token 和本机许可确认。绝对路径留在原生端；这一步不运行真实探针，状态仍是 `NOT_RUN/EXTERNAL`。
2. 开放包提交后，“导出与外部 CLI”可以启动 `IMPORT_PROJECT`、`PACK_ATLAS` 或 `EXPORT_BINARY`。每个 job 都绑定当前项目 revision、本轮 export、输入哈希、独立输出目录和一次性人工确认。
3. job 异步经过原生输入、consent 准备、人工确认和执行阶段；UI 只轮询有界状态，不自行声称结果。
4. 宿主保存脱敏 provenance，UI 只显示 job/operation ID、path token、输出相对路径、输出哈希和授权结果，不返回绝对路径或激活信息。

当前代码库没有附带一次真实合法 4.2.43 CLI/Editor 往返证据，因此候选的初始外部状态仍是 `NOT_RUN/EXTERNAL`。若 CLI 缺失、版本不是 4.2.43、人工取消、超时或输出不满足合同，单个 job 保持 `NOT_RUN` 或 `FAILED`；系统不会回退到自研专有 writer。单个 job 的 `SUCCEEDED` 只证明该轮受限 CLI 操作和输出 provenance 合同通过，Editor 人工视觉验收仍需单独记录。

## 7. Editor 人工验收建议

若用户具有合法 Spine 4.2.43 环境，可对本轮包执行以下外部检查：

1. 在 Editor 中导入/打开本轮生成的工程，不使用其他 patch 自动升级保存。
2. 检查 canvas、骨骼父子关系、slot/draw order、attachment 路径和主武器 socket。
3. 逐个播放固定十动作，检查循环动作、起落地、冲刺轮廓、三个攻击和 hit/death。
4. 逐个核对 `attack_01` 至 `attack_03` 的唯一命中帧。
5. 将 Editor/CLI 精确版本、可执行文件哈希、输入包哈希、输出哈希和人工结论记录为本轮外部 evidence。

没有上述真实证据时，即使 JSON 可解析，也只能是 `EXPORTED_UNVERIFIED`。

## 8. 这不是游戏发布审批

角色资料包不包含攻击伤害、碰撞体、取消窗口、输入缓冲、状态机、相机、敌人 AI、关卡或游戏引擎控制器。关键姿势和 hit marker 只提供动画/玩法对齐点，不能替代设计师和程序员的游戏逻辑。

产品的“导出成功”也不等于软件“发布授权”。当前候选未签名，Release Gate 尚未得到组织授权；任何公开分发、销售或兼容性宣传都需要独立的许可、签名、真实 Spine 往返和发布审批。
