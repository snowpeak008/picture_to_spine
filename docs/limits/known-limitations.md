# 已知限制与项目边界

## 1. 能做什么

FlashToSpine V1 可以辅助一名技术美术或 2D 动画师完成：本地图片预检、母版与风格确认、分层修正、Rig 骨骼/slot/pivot/socket 调整、固定十动作的 MotionSpec/BOM/PromptPack、关键姿势图片导入/对齐与人工审核、动画轨道编辑、关键姿势与三个攻击命中帧编辑/审核，以及开放格式导出。

更准确的定位是“角色 Spine 生产资料与人工门禁工作台”，不是端到端游戏制作器，也不是无人值守的单图转成品服务。

## 2. 不能从一张图恢复的信息

单张合成图只包含一个视角和一个时刻。它通常没有：

- 被头发、衣服、身体或武器遮挡的真实像素；
- 关节背面的结构和转身后的形状；
- 大幅透视变化、衣摆翻转、武器挥舞残影或表情替换；
- 骨骼中心、mesh 拓扑、权重、动作节奏和接触时刻的唯一真值；
- 不同攻击的关键轮廓、蓄力/释放/收招设计。

因此工具必须依赖用户补充动作关键帧图片和动作描述，并保留分层、Rig、pose、hit 的人工修正/审批。提示词能帮助外部 AI 或美术生产缺失素材，但不能保证身份一致、解剖正确或可绑定。

## 3. 固定内容范围

- 仅二次元类人角色；不支持任意生物、载具或 3D 角色。
- 仅横版侧视；不承诺正面、背面、俯视或多方向角色集。
- 仅一件主武器和一个主武器 socket；双武器、换装武器组和动态武器拓扑不在 V1。
- 动作只能是 `idle/run/jump/fall/dash/attack_01/attack_02/attack_03/hit/death`。
- 只有三个 attack 动作拥有 hit marker；每个攻击恰好一个。
- 不生成额外连段、技能、投技、受身、攀爬、游泳或多阶段 Boss 动作。

## 4. 图像和 AI 边界

产品只接收用户本地提供的 PNG/JPEG/WebP 和文本，不生成图片，不内置图像模型，不调用公有图像 API，也不自动上传角色素材。默认导入只接受 8-bit 图片，并受 64 MiB、16,777,216 像素、完整解码和压缩比预算约束。

PromptPack 是确定性的文本辅助资料。`networkCallsMade: 0` 只说明 PromptPack 合成本身没有联网，不证明用户之后使用的外部生成工具安全、合法或质量合格。

用户自托管的私有远程 GPU 是独立可选边界，只能产生分层、Rig 或动作曲线**候选**，不能生图或代替人工 gate。当前已实现 profile 管理、领域合同、状态机、隔离存储、deterministic mock 和 Windows Credential Manager adapter，但真实 HTTPS transport、TLS/SPKI 连接和远程 job UI/host 尚未接入。当前宿主不读取远程 secret、不建立连接，真实远程 GPU 必须保持 `NOT_RUN/EXTERNAL`；mock 测试通过不能外推到真实端点、模型或删除证明。

## 5. Rig 和动画限制

- 内置默认分层/rig 候选需要人工检查，不能把视觉上“像”当作生产可用。
- 当前导出器拒绝多骨骼权重；每个顶点必须 100% 刚性绑定单一骨骼。
- mesh、weight 和约束能力不是 Spine Professional 全功能编辑器的替代品。
- Rig UI 可修改现有骨骼的父级、X/Y、旋转、X/Y 缩放，slot 的骨骼绑定/draw key，以及 pivot/socket；V1 不提供骨骼增删、mesh 顶点、weight painting 或约束编辑。
- 关键姿势图片对齐只保存 Ground Y 与统一 Scale 的审批预览元数据；它不修改 CAS 像素、不裁切/旋转、不做骨骼 retarget，也不会自动改写 AnimationSet。
- 内部 Pixi/Rig IR 预览只模拟刚性单骨 attachment 随骨骼变换的工作视图；不模拟 mesh 变形、多骨骼蒙皮、完整 Spine 约束，也不保证与 Spine Runtime/Editor 像素一致。
- 整数 tick、曲线和 marker 能描述动画，但不能自动判断打击感、可读性、取消窗口或玩法平衡。
- 命中 marker 只允许在对应 MotionSpec 的 `contact` phase 内移动并绑定唯一主武器 socket；它不是 hitbox、伤害或取消窗口。V1 未提供 transition 图编辑、可视化撤销/重做历史或批量 retarget。
- PSD 是最小分层交换文件，不是 Photoshop/Spine 工程的完整保真替代；隐藏区仍可能需要人工绘制。

## 6. Spine 与输出限制

唯一兼容目标是精确的 Spine Editor/Professional CLI **4.2.43**。4.2.42、4.2.44、其他 4.2 patch、`latest` 或版本范围均未验证。

内置 writer 只输出 Rig IR、最小 PSD、透明 PNG、`character.spine.json`、atlas input manifest、PromptPack、兼容清单和 checksums。它永远不生成 `.atlas`、`.spine` 或 `.skel`。

开放包与 CLI 输出的规范化目标路径必须位于整个 `%LOCALAPPDATA%\FlashToSpine` 私有数据根之外。该限制覆盖未来新增的私有子目录，不只覆盖当前已知的 `projects`、`cas` 或 `staging`。

专有格式只允许用户本地合法的 Spine Professional/适用 Enterprise 4.2.43 生成。产品不捆绑 Editor、CLI、Runtime、激活信息或许可证。当前设置页、导出页和宿主已接入 JSON→`.spine`、images/settings→`.atlas`、`.spine`→`.skel` 三类受限异步 job，但代码库没有附带真实 4.2.43 运行证据。选择 CLI 或 synthetic 测试都不能升级状态；没有真实本轮 job provenance 与 Editor evidence 时，即使开放包内部检查通过，状态仍是 `EXPORTED_UNVERIFIED/EXTERNAL`。

V1 选择“仅 Spine Editor”为集成目标；Unity、Godot、Cocos 和自研 Runtime 适配都不在 V1。输出不能直接拖进游戏引擎即获得完整角色控制器。

## 7. 不能制造完整重度动作游戏

本项目不能独立制作一款 2D 重度动作游戏。它不负责：

- 输入、状态机、连招分支、取消和缓冲；
- hurtbox/hitbox、伤害、硬直、无敌帧和击退；
- 敌人 AI、关卡、相机、物理、网络同步和存档；
- 引擎运行时、资源加载、性能预算和平台发布；
- 音效、特效、镜头震动和最终打击反馈；
- 大规模角色、多武器、多方向或皮肤生产管理。

它可以做到的上限，是为横版二次元单武器角色提供一套经过人工门禁的十动作 Spine 生产候选和外部 Editor 验证输入。它可作为更大游戏管线的“角色动画资产准备”环节，但后续仍需资深动画师、技术美术、游戏程序、设计师和实际引擎管线。

因此不应把《Dead Cells》《Hollow Knight》或任何商业动作游戏当作本工具可复制的成品参考。最多只能把这些作品视为动作轮廓、读帧和打击节奏研究对象；其玩法系统、引擎、内容规模和美术生产远超本项目边界。

## 8. 本地存储和安全限制

生产入口使用 DPAPI CurrentUser 密钥、HMAC ProjectHead、signed revision sidecar 和高水位 anchor 发现本地篡改、回滚与分叉。其边界是当前 Windows 用户上下文中的完整性保护：

- 它不是自然人数字签名、组织证书或不可否认性证明；
- 拥有同一用户会话和密钥解封能力的高权限攻击者仍超出单机应用可完全防御的范围；
- DPAPI 密文通常不可直接迁移到另一用户/机器；当前没有受支持的跨机密钥迁移；
- 旧 unsigned/unanchored 项目会 fail-closed；没有自动迁移或忽略校验开关；
- 中断提交只允许沿 HMAC 有效、直接连续的 signed sidecar authenticated roll-forward；它不能恢复任意历史版本、绕过高水位、接受分叉或充当 schema migration；
- 项目数据和 CAS 不是云备份，用户必须自行做关闭应用后的整根备份。

## 9. 平台、打包和发布限制

- 当前目标是 Windows 11 x64；需要系统 WebView2 Evergreen Runtime。
- 桌面端是原生 Win32 + `webview2-com` 直接宿主；`apps\desktop\src-tauri` 仅是历史目录名，不表示当前使用 Tauri。
- 便携 Core 不安装 WebView2，也未包含 AppContainer AI Worker。
- 当前内部候选未代码签名，clean-VM 双 runner 验证不完整。
- 本机 WebView2 启停探针通过时覆盖启动页 DOM 就绪、窗口/运行时/正常关闭；原生图片对话框、六类人工门、重开和导出的完整 GUI 业务链仍需人工 E2E 验证。
- 发布依赖政策只允许经审计的 MIT、Apache、BSD 等宽松许可；外部专有 Spine/WebView2 不属于开源发行依赖。
- 当前没有发布授权。构建、测试或文档完成不能自动批准公开销售、分发或兼容性宣传。

## 10. 状态和证据限制

- 静态合同 `PASS` 不等于真实 Spine Editor 往返 `PASS`。
- mock `PASS` 不等于真实私有 GPU `PASS`。
- `NOT_RUN`、`UNVERIFIED` 和 `EXTERNAL` 不能折算成成功率。
- JSON 可解析不等于动画视觉正确；内部预览可播放不等于游戏手感正确。
- 没有真实目标域数据集和人工评测前，不公布任意单图成功率、商业级质量百分比或重度动作产能承诺。

这些限制是产品合同的一部分。若未来增加多武器、其他动作、游戏引擎、真正多骨骼蒙皮、远程执行或其他 Spine patch，必须建立新的需求、ADR、schema、许可审计、测试矩阵和实际证据，不能只修改 UI 文案。
