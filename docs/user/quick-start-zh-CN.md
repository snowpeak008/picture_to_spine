# FlashToSpine 快速开始

## 1. 先确认它是什么

FlashToSpine 是一款面向 Windows 的闭源商业 **Production Assist**。它把用户合法持有的二次元类人、横版侧视、单一主武器角色图片和动作关键帧图片，整理成可人工修正、可审批、可追踪的 Spine 4.2.43 生产资料。

它不会生成图片，不调用公有图片生成服务，也不会从一张图自动制造完整的重度动作游戏。游戏玩法、碰撞、伤害、连招状态机、敌人逻辑和 Unity/Godot/Cocos/自研 Runtime 集成都不在 V1 中；V1 的唯一外部目标是 Spine Editor/Professional CLI 4.2.43。

桌面程序由原生 Win32 + `webview2-com` 直接宿主系统 WebView2。仓库中的 `apps\desktop\src-tauri` 只是历史目录名，当前程序不是 Tauri 应用。

## 2. 运行前准备

- Windows 11 x64；当前候选没有完成 Windows 10 发布验证。
- 系统已安装 Microsoft Edge WebView2 Evergreen Runtime。便携包不会替用户安装或更新它。
- 至少准备一张 8-bit PNG、JPEG 或 WebP 角色图。默认导入上限为 64 MiB、16,777,216 像素；还会执行完整解码和压缩比检查。
- 角色必须是二次元类人、横版侧视，并且只描述一件主武器。
- 若要生成 `.atlas`、`.spine` 或 `.skel`，用户还必须自行合法安装 **Spine Professional 或适用的 Enterprise 4.2.43**。FlashToSpine 不捆绑 Editor、Runtime、CLI、激活信息或许可。

## 3. 启动

内部便携候选可双击项目根目录的 `FlashToSpineLauncher.exe`。打包目录中的等价入口是 `dist\FlashToSpine-Core\FlashToSpine.exe`。二进制是否对应当前源码，以包内 `package-manifest.json` 的源码绑定和 `npm run test:package` 结果为准。

开发环境可双击 `FlashToSpine-开发入口.cmd`。该脚本只查找已有可执行文件并启动；它不会安装依赖或提升权限。若提示 `F2S-LAUNCH-001`，先在项目根目录执行：

```powershell
npm run build:ui
npm run build:core
npm run package:core
npm run test:package
```

当前便携候选未做代码签名，签名状态必须保持 `NOT_RUN/EXTERNAL`。它不是已授权公开发布的安装包。

## 4. 完成一套角色资料

### 4.1 创建项目并导入母版

1. 在“总览”创建本地项目。
2. 进入“导入与母版”，通过 Windows 原生文件选择器选择本地图片。
3. 核对格式、尺寸、8-bit、完整解码和 SHA-256 结果，再把候选提升到本地 CAS。
4. 填写侧视风格、轮廓、配色、身份说明和唯一主武器语义。
5. 创建母版候选，在原生确认框中人工批准或写明原因退回。

选择文件、通过解码和进入 CAS 都不等于审批。只有与当前图片哈希和候选 revision 绑定的人工批准才会打开下游工作台。

### 4.2 修正并批准分层

1. 在“分层工作台”初始化 LayerSet。
2. 检查图层名称、顺序、可见性和重组预览。
3. 通过添加、删除、重排和遮罩笔划修正分层。隐藏区域不能从原图可靠推断时，应由美术人员提供或修正素材。
4. 处理阻断问题后，人工批准当前 LayerSet。

修改已批准的母版或分层会使依赖它的后续批准失效。不要通过手改项目 JSON 绕过重审。

### 4.3 调整并批准 Rig

1. 在“Rig 工作台”从当前已批准 LayerSet 创建候选。
2. 调整骨骼静止位姿的 X/Y、旋转、X/Y 缩放和父子关系。
3. 按图层修正 slot 的绑定骨骼与唯一 draw key，并调整 pivot 和唯一主武器 socket。
4. 检查 mesh、刚性权重和约束摘要；这些项目在 V1 中只读，不是 Spine 的完整蒙皮编辑器。
5. 在预览和诊断无阻断后，人工批准 Rig。

当前内置 Spine writer 只接受每个顶点 100% 绑定到单一骨骼的刚性权重；真正的多骨骼蒙皮会以 `MULTI_BONE_WEIGHTS_UNSUPPORTED` 阻断导出。

### 4.4 生成十动作内容计划并导入关键姿势图

“动作内容与素材计划”固定包含以下十个动作，名称和顺序不能改写：

```text
idle
run
jump
fall
dash
attack_01
attack_02
attack_03
hit
death
```

1. 初始化 MotionContent。
2. 逐动作检查 MotionSpec、素材 BOM 和 PromptPack。
3. 把 PromptPack 中的正向/负向提示词复制到用户自行选择的外部图片工具；FlashToSpine 本身不会生图。
4. 将生成或手绘完成的动作关键姿势图片保存到本机，再逐张选择、预检并提升为候选。
5. 用每张绑定图的 `Ground Y` 与统一 `Scale`（0.01x–100x）对齐参考姿势。它只改变审批绑定的预览元数据，不修改 CAS 原图、不裁切/旋转、不做自动骨骼 retarget；保存后该图片及受影响动作需要重审。
6. 核对图片预览、用途、动作、姿势、尺寸、对齐和哈希后，逐张通过原生确认框批准。

如果当前构建没有显示待批准图片的权威预览，或者预览与哈希/用途无法对应，请不要批准；保持该素材未批准并报告诊断信息。

### 4.5 编辑动画并审核关键姿势、命中帧

1. 从已批准 Rig 和 MotionContent 创建十动作 AnimationSet。
2. 逐动作编辑骨骼 translate/rotate/scale、slot color/draw-order 轨道和整数 tick 关键帧。
3. 检查预览并为每个动作设置 review pose marker；marker 只能选择当前真实 track 已存在的关键帧 tick，它不是玩法事件。
4. 分别人工批准十个动作的关键姿势。
5. 对 `attack_01`、`attack_02`、`attack_03` 编辑唯一命中 tick；它必须位于该动作 MotionSpec 的 `contact` phase 内，并绑定当前唯一主武器 socket。可直接输入整数 tick，或在播放头位于 `contact` 区间时使用当前播放头。
6. 先批准对应关键姿势，再独立批准三个命中帧。调整命中 tick/socket 只会使该动作的 Hit 审批失效，不会撤销已通过的 Pose 审批。

内部 Pixi 预览直接读取 Rig IR，只显示刚性单骨 attachment 随骨骼变换的工作视图。它不模拟 mesh 变形、多骨骼蒙皮、Spine Runtime 的全部约束或 Spine Editor 的像素级结果。预览通过不表示 Editor 往返通过。

### 4.6 预检并导出开放格式包

1. 进入“导出与验证”。
2. 执行重新预检，处理所有 `BLOCKED` 项。
3. 预检通过后选择一个不位于整个 `%LOCALAPPDATA%\FlashToSpine` 私有数据根中的本地目录。不能把开放包写进该根下的任何子目录。
4. 点击“选择目录并提交开放包”。工具会创建新的、不可覆盖的导出子目录。

内置包包含：

- `rig-ir.json`
- `character.psd`（最小分层交换文件）
- `character.spine.json`（Spine 4.2.43 JSON candidate）
- `images/layer-*.png`（透明 attachment PNG）
- `atlas-input-manifest.json`
- `prompt-pack.json` 和 `prompt-pack.md`
- `compatibility-manifest.json`
- `checksums.sha256`

导出成功状态是 `EXPORTED_UNVERIFIED`，不是“已通过 Spine”。包目录不可变；再次导出会生成新的 export ID 和目录。

## 5. 可选的本地 Spine 4.2.43 步骤

`.atlas`、`.spine`、`.skel` 只能由用户本地、合法的 Spine Professional/Enterprise 4.2.43 Editor 或 `Spine.com` 生成。当前 UI 与原生宿主已经接入受限的异步 CLI job，但仓库和内部候选没有附带一次真实 4.2.43 运行结果；初始状态必须保持 `NOT_RUN/EXTERNAL`。

1. 在“设置”中选择本机规范路径下的 `Spine.com`，并在 Windows 原生确认框中确认用户具有适用的 Professional 许可。应用只向 WebView 返回 path token 和可执行文件 SHA-256，不返回绝对路径，也不读取激活信息。
2. 先在“导出与外部 CLI”提交本轮不可变开放包。开放包提交成功仍是 `EXPORTED_UNVERIFIED`。
3. 在同一应用会话中选择一个操作：
   - “生成 `.spine`”：把本轮 `character.spine.json` 导入新的外部目录；
   - “生成 `.atlas`”：使用本轮 `images`，并由用户原生选择 pack settings JSON；
   - “生成 `.skel`”：由用户原生选择一个 `.spine` 项目，再输出到新的外部目录。
4. 为输出选择一个不位于开放包或产品私有数据根中的新目录，并核对本轮 operation、输入哈希、参数形状和 consent binding。
5. 在原生确认框中逐次批准该 job。界面会轮询 `QUEUED`、`AWAITING_NATIVE_INPUT`、`PREPARING_CONSENT`、`AWAITING_HUMAN_CONFIRMATION`、`RUNNING`，直到 `SUCCEEDED`、`FAILED` 或 `NOT_RUN`。
6. 只有运行前后都精确观测到 4.2.43，预期扩展真实产生，并且每个输出都通过 provenance 哈希授权，该 job 才能显示 `SUCCEEDED`。这仍不等于人工 Editor 视觉验收通过。

选择或配置 `Spine.com` 本身不会执行探针，也不会把状态升级为成功。应用重启后，上一会话的开放包授权不会自动成为新的 CLI job 输入；需要重新提交或在当前会话完成操作。

任何 4.2.42、4.2.44、其他 patch、版本范围或 `latest` 都不满足本项目的兼容声明。不要用其他版本覆盖输出后仍沿用 4.2.43 的状态。

## 6. 私有远程 GPU 的当前边界

“私有 GPU 设置”可以导入、校验、显示和停用本地 profile。领域合同、状态机、隔离存储、deterministic mock 和 Windows Credential Manager adapter 已实现，但当前桌面端没有接入真实 HTTPS transport 或远程 job 流程。

- 导入或启用 profile 不会连接端点；
- 当前宿主不读取 Credential Manager secret，WebView 也不接收 secret；
- 当前网络尝试计数保持 0，真实能力为 `NOT_RUN/EXTERNAL`；
- mock 的测试 `PASS` 不能当作真实私有端点、模型或删除收据的 `PASS`；
- 公有 AI provider、图片生成、自动 fallback 始终不允许。

## 7. 状态词怎么理解

| 状态 | 含义 |
| --- | --- |
| `BLOCKED` | 当前事实不满足操作前置条件，未输出 |
| `CONTRACT_VERIFIED` / `CONTRACT_AVAILABLE` | 内置开放格式静态合同通过，不等于 Editor 实测 |
| `EXPORTED_UNVERIFIED` | 文件已经写出并校验内部清单，但没有完成合法 Editor/CLI 往返 |
| `EXTERNAL` | 能力属于用户工具或外部环境，FlashToSpine 未执行或不能自证 |
| `NOT_RUN` | 本轮没有执行；不能当作通过 |
| `UNVERIFIED` | 有候选或观测，但证据不足以升级为已验证 |
| `PASS` / `FAIL` | 只针对明确执行过的测试或检查 |

## 8. 本地数据与安全提醒

“环境诊断”页可通过 Windows 原生保存对话框导出脱敏 JSON。报告只包含产品/能力状态、计数、脱敏项目摘要、文件名、字节数和 SHA-256，不包含角色图片、PromptPack 正文、Credential Manager secret、Windows 用户名或绝对路径。导出诊断成功不表示 Spine、远程 GPU、Worker、clean-VM 或签名已经验证。

项目、CAS、暂存、恢复记录以及 WebView2 用户数据位于 `%LOCALAPPDATA%\FlashToSpine`；浏览器缓存不会写在 exe/便携包旁。生产入口使用 Windows DPAPI CurrentUser 保护的本地完整性密钥，对项目头、revision 链和高水位 anchor 做 HMAC 校验。

- 关闭程序后再备份整个 `%LOCALAPPDATA%\FlashToSpine`，不要只复制 `head.json`。
- 不要编辑、删除或重算 `head.json`、`*.head.json`、anchor 或 DPAPI 密钥文件。
- 提交时先形成不可变 signed sidecar，再发布高水位 anchor 和 `head.json`。若进程在发布中断，安全 store 只会沿 HMAC 有效、revision 直接相邻的 signed sidecar 向前恢复；这叫 authenticated roll-forward，不是接受任意旧 head、回滚或分叉。
- 旧版 unsigned 项目会 fail-closed；当前产品没有自动迁移或“忽略完整性后打开”开关。
- 这套机制用于发现本地篡改、回滚和分叉，不是自然人电子签名，也不提供不可否认性。

更完整的审批和导出说明见 [approval-and-export.md](approval-and-export.md)，产品边界见 [known-limitations.md](../limits/known-limitations.md)。
