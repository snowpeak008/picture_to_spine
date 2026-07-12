# FlashToSpine 运维手册

## 1. 适用范围与当前交付状态

本手册面向内部维护者。当前交付物是 Windows x64 闭源商业 Production Assist 的**内部便携候选**，不是已授权公开发布版。便携包不安装服务、不提升权限、不下载依赖，不包含 Spine Editor/Runtime、模型权重、Python、CUDA 或 AppContainer AI Worker。

桌面交付采用原生 Win32 窗口和 `webview2-com`，直接宿主系统 WebView2 Evergreen Runtime。React/TypeScript/PixiJS 由 esbuild 构建并嵌入 exe，Rust 是 IPC、revision、审批、CAS 和文件写入的权威端。`apps\desktop\src-tauri` 只是历史目录名；当前二进制不包含 Tauri、Wry 或 Tauri 插件运行时。

必须始终区分：

- 构建/测试 `PASS`：某条本地检查实际执行成功；
- `UNVERIFIED_CLEAN_VM`：尚无两台独立干净 Windows runner 的同一来源复现证据；
- `NOT_RUN/EXTERNAL`：Spine、远程 GPU、签名或其他外部条件未执行；
- 发布授权：独立治理决定，不能由测试脚本或文档产生。

## 2. 工具链与构建

仓库固定候选为 Node 24.15.0、npm 11.12.1、Rust/Cargo 1.96.0，目标 `x86_64-pc-windows-msvc`。MSVC、Windows SDK 和 WebView2 是系统前置，探测器不得自动安装或升级它们。

从项目根目录执行：

```powershell
npm run bootstrap:check
npm run format:check
npm run lint
npm run typecheck
npm test
npm run test:integration
npm run test:spine
npm run build:ui
npm run build:core
npm run package:core
npm run test:package
```

最终打包后可另行运行 `npm run test:webview-local`。它是显式、本机、隐藏窗口的 WebView2 启停探针；不探测精确 Runtime 版本，不属于 headless `test` 或 `release:verify`，也不能代替 clean-VM。`npm run release:verify` 会执行许可清单、CI 合同、Spine 静态测试、Core 打包和包验证；它的名字不表示自动授予 release。命令使用 `Cargo.lock`/`package-lock.json`，Core Rust 构建采用 `--locked --offline`。若本地缓存缺失，应按组织批准的依赖获取流程处理，不得让发布脚本静默联网。

## 3. 包与入口

- 根入口：`FlashToSpineLauncher.exe`
- 内部便携包：`dist\FlashToSpine-Core`
- 包入口：`dist\FlashToSpine-Core\FlashToSpine.exe`
- 开发入口：`FlashToSpine-开发入口.cmd`
- 开发启动结果：运行 `FlashToSpine-开发入口.cmd` 后才生成 `evidence\M01\F2S-DEV-M01-004\F2S-WU-M01-004-01\launcher-result.json`

`npm run package:core` 会重建 `dist\FlashToSpine-Core` 并把同一可执行文件复制为根入口。package manifest 绑定当前源码树和两个 lock 文件；`npm run test:package` 校验该绑定、包文件白名单、SHA-256、PE 头和 `--smoke` 结果。smoke 不启动 WebView2，因此 `PASS_WITHOUT_WEBVIEW2_PROBE` 不能证明图形运行时可用。仅存在 exe 或旧 `launcher-result.json` 不能证明它对应当前源码。

最终打包后可单独运行 `npm run test:webview-local`。宿主先使用“正在验证界面”标题，只有内部文档导航完成且 React 根节点和 `.app-shell` 可见于 DOM 后才发布探针期望的最终标题。因而探针证明当前机器上的启动页 DOM、预期 Win32 顶层窗口、WebView2 Chrome/Render 子窗口、响应性与 `WM_CLOSE` 清理合同成立；报告仍固定为 `LOCAL_RUNTIME_ONLY`、`cleanVm=false`，不得据此写成 clean-VM 或 GUI 业务全链路通过。

若内部导航或 DOM 就绪失败，宿主必须显示 `F2S-BOOT-NAVIGATION` 或 `F2S-BOOT-DOM` 启动错误并退出，不能以最终标题继续显示空白/黑色窗口。导航白名单只接受 `about:blank` 和本次嵌入 HTML 对应的精确 `NavigateToString` data URI，不接受任意 `data:` 或网络地址。

当前包未签名，`package-manifest.json` 和 smoke 必须保持 `NOT_RUN_EXTERNAL`，直到真实签名流程和验证证据存在。不要手改 manifest 将状态升级。

## 4. 本地数据布局

生产数据根为 `%LOCALAPPDATA%\FlashToSpine`：

| 路径 | 内容 | 运维规则 |
| --- | --- | --- |
| `projects\<projectId>` | `head.json`、不可变 revisions、签名 revision sidecar | 不手改、不回滚单个文件 |
| `projects\security\anchors` | 每项目最高 revision 的 HMAC anchor 投影 | 只允许由有效 signed sidecar 做 authenticated roll-forward；禁止手工回退 |
| `cas` | 以内容哈希寻址的图片和 manifest | 不按文件名猜用途，不单独清理 |
| `staging` | 尚未消费的本地图片暂存 | 故障时先隔离/备份，不在程序运行中删除 |
| `export-recovery` | 开放包已提交但项目历史提交失败的恢复记录 | 与对应导出包一起保存 |
| `security\project-integrity-key.dpapi` | DPAPI CurrentUser 保护的 256-bit 项目完整性密钥密文 | 不复制明文、不编辑、不重建覆盖 |
| `WebView2` | 系统 WebView2 的用户数据与缓存 | 由宿主显式放入本地私有根，不能写在 exe/便携包旁；关闭程序后方可维护 |

完整性密钥 ID 为 `dpapi-current-user-v1-<key指纹前8字节hex>`。密钥由当前 Windows 用户的 DPAPI 解封；项目头、前一 revision 链接、signed sidecar 和高水位 anchor 使用 HMAC-SHA256 校验。

## 5. 备份、还原与完整性

### 5.1 备份

1. 正常关闭 FlashToSpine，确认没有正在导入、提交或导出的操作。
2. 复制整个 `%LOCALAPPDATA%\FlashToSpine` 到组织批准的加密备份位置。
3. 对备份根生成外部 SHA-256 清单，并记录 Windows 用户/机器上下文、应用版本和时间。
4. 保持备份只读；不要只备份 `projects` 或只备份 `head.json`。

DPAPI CurrentUser 密文通常不能在另一个 Windows 用户或另一台机器上解封。当前没有受支持的密钥导出/跨机项目迁移功能。需要可移交素材时，应另行保存已导出的开放格式包；它不替代可继续编辑的完整项目备份。

### 5.2 还原

1. 保留故障现场的只读副本。
2. 在同一 Windows 用户和可解封同一 DPAPI 密钥的环境中，还原完整数据根。
3. 启动后只执行“打开项目”，不要先修改项目文件。
4. 打开项目时允许安全 store 完成内置恢复；不要在首次打开前复制或重写单个 head/anchor。若报告 MAC、anchor、chain、CAS hash、rollback 或 fork 错误，停止操作并保留全部文件。

安全 store 会在读取 manifest/CAS 前验证 ProjectHead、anchor、signed sidecar 和完整 revision 链。提交顺序是：先落盘不可变 signed sidecar，再发布高水位 anchor，最后发布 `head.json`。进程可能在任一投影发布之间退出，因此读取端支持以下受限恢复：

- 只有有效 genesis sidecar、尚无后续 revision 时，可以重建缺失的 genesis head/anchor；
- head 或 anchor 只相差一个 revision 时，必须由对应 signed sidecar、HMAC 和直接前驱关系共同证明，才能把较旧投影向前推进；
- 两个投影都停在旧 revision，但存在一个或多个连续、有效的 signed successor 时，可以逐 revision roll-forward；
- unsigned orphan revision 会被隔离，不会被当作已提交事实。

这是中断提交的 authenticated roll-forward，不是 rollback、版本迁移或“选择任意 revision 打开”。sidecar 篡改、链断裂、分叉、投影相差超过一个 revision、删除高水位后伪造旧状态或 HMAC 不匹配仍会 fail-closed。不能通过切换到 legacy store、删除 anchor 或重新签一个当前文件来“修复”。

### 5.3 旧 unsigned 项目

生产入口不接受历史 unsigned 项目，当前没有自动迁移、批量签名或忽略完整性开关。维护者应：

1. 保留原目录和哈希，不在原地修改；
2. 标记 `UNSUPPORTED_UNSIGNED_PROJECT / NOT_MIGRATED`；
3. 等待经过评审、copy-on-write、有备份和人工确认的专用迁移工具；
4. 在该工具不存在时，不得把测试用 `FsProjectStore::new` 当作生产恢复路径。

## 6. 导出故障处理

- 导出根必须已存在、可写，且其规范化路径不能位于整个 `%LOCALAPPDATA%\FlashToSpine` 私有数据根内。不要把未枚举的新私有子目录误当作允许的导出位置。
- 包使用 `.f2s-staging` 子目录写入，核对清单后原子改名。已有同名 export/staging 会拒绝覆盖。
- 若预检 `BLOCKED`，回到对应工作台重审，不手改 PublishSnapshot 或 compatibility manifest。
- 若提示“immutable export completed ... project history commit failed”，导出包可能已经完整。保存包、`export-recovery\<exportId>.json` 和项目数据根；不要重复占用原目录。
- `checksums.sha256` 不匹配时，把整个包视为失败/被修改。不要只重算 checksums 使其“通过”。

开放包没有 `.atlas`、`.spine`、`.skel` 是正常现象，不是丢文件。

内部 Pixi 预览只消费当前 Rig IR，并按刚性单骨 attachment 变换提供工作视图。它不模拟 mesh 变形、多骨骼蒙皮、完整 Spine 约束或 Editor/Runtime 像素结果，不能作为外部兼容证据。

## 7. Spine 4.2.43 外部运维

Spine Editor/Professional CLI 完全由用户安装和许可。维护者不得：

- 把 `Spine.com`、Editor、Runtime、激活文件或许可密钥放进发行包；
- 自动下载、更新或激活 Spine；
- 接受 4.2.42、4.2.44、版本范围或 `latest` 冒充目标版本；
- 在没有本轮可执行文件哈希、精确版本、操作确认和输出 provenance 时标记为 VERIFIED；
- 在 CLI 失败时启用内置 `.atlas/.spine/.skel` writer。

设置页、导出页与原生宿主已接入 CLI 状态、选择和三类异步 job：

- `IMPORT_PROJECT`：从本轮开放包的 `character.spine.json` 生成独立 `.spine`；
- `PACK_ATLAS`：从本轮开放包的 `images` 和用户选择的 settings 生成 `.atlas`；
- `EXPORT_BINARY`：从用户选择的 `.spine` 生成 `.skel`。

CLI 安全适配器约束本地合法 `Spine.com` 的规范路径、文件哈希、精确版本、固定参数、超时、输出预算和独立输出目录。配置可执行文件只建立本地 HMAC 保护的路径记录与许可确认，不运行真实探针。每个 job 必须重新绑定当前 export、operation、输入哈希和一次性原生人工确认；UI 异步轮询状态，provenance 保存在私有本地目录。

当前仓库没有附带真实合法 4.2.43 CLI/Editor 往返结果，初始状态必须保持 `NOT_RUN/EXTERNAL`。只有实际 job 完成前后精确版本探测、生成预期专有扩展且逐文件 provenance 授权后，该 job 才能为 `SUCCEEDED`。静态或 synthetic 测试不能替代真实 Editor/CLI 往返，CLI job 成功也不能替代 Editor 人工视觉验收。

## 8. 私有远程 GPU

远程 GPU 是**可选、默认禁用、仅用户控制的私有 HTTPS 端点**。配置示例在 `config\remote-gpu.example.json`，允许的候选方法只有：

- `LAYER_SEGMENTATION_CANDIDATE`
- `RIG_PROPOSAL_CANDIDATE`
- `MOTION_CURVE_CANDIDATE`

配置要求精确 origin/port、证书 SPKI SHA-256、组织身份 SHA-256、模型 manifest 白名单、上传/响应预算和 Windows Credential Manager target。配置文件只保存 credential 引用，不得保存 token、密码或 cookie。公有 AI 提供商、图片生成和任意方法均被拒绝。

当前已实现 profile 导入/停用、领域合同、状态机、隔离存储、deterministic test mock 和 Windows Credential Manager generic credential adapter。Credential Manager adapter 只接受严格的 `FlashToSpine/RemoteGpu/<profile-id>` target，并在当前 Windows 用户凭据域保存 secret；secret 不进入项目、序列化 DTO 或日志。

真实 HTTPS transport、TLS/SPKI 连接和远程 job UI/host 流程尚未接入。当前宿主不会读取远程 secret，`credentialConfigured` 保持未检查，网络尝试计数为 0，真实远程能力必须保持 `NOT_RUN/EXTERNAL`。填写或启用 profile 不等于已经建立连接，也不允许把 mock 的 `PASS` 当作真实端点、模型执行或删除收据的 `PASS`。

维护远程配置时：

1. 先保持 `enabled: false` 审核 profile；
2. 在 Windows Credential Manager 中独立配置/轮换凭据，日志只记录 target 名；
3. 在真实 transport/job 尚未接入期间，不尝试从 UI 发起传输，也不把领域 service 或 mock 当作运维入口；
4. 未来每次真实传输必须展示精确文件、用途、哈希、字节数和目标 profile，并获得一次性人工批准；
5. 未来返回结果必须先校验 receipt、大小、哈希、模型 manifest，再进入 quarantine；取消或失败不代表远端已删除数据；
6. 撤销时禁用 profile，并在 Credential Manager 中删除对应凭据；保留审计记录但不保留秘密。

## 9. 诊断与支持材料

“诊断”页报告宿主实际观测到的 IPC、隔离图片解码、项目完整性、Worker、私有远程、Spine CLI/Editor 和本轮网络计数。未探测项保持 `UNVERIFIED/EXTERNAL`。

当前 UI 与宿主已接入“选择位置并导出脱敏 JSON”。报告只在用户通过原生保存对话框确认后写入，返回文件名、字节数和 SHA-256；它不含角色图片、PromptPack 正文、secret、用户名或绝对路径。诊断导出成功只证明脱敏报告合同执行，不会把 Spine、远程 GPU、Worker、clean-VM 或签名升级为已验证。人工收集支持材料时只包括：

- 应用/包 manifest 和 SHA-256；
- `launcher-result.json`；
- 失败代码、时间、项目 ID、revision 和**截断后的**对象哈希；
- 对应 export recovery 或外部 CLI/remote report 的脱敏副本；
- 实际运行过的测试命令及退出码。

不要收集角色原图、CAS 内容、完整本地绝对路径、Windows 用户名、Credential Manager 秘密、Spine 激活信息、DPAPI 密钥密文/明文或私有端点 token。需要路径上下文时使用环境变量和相对路径表达。

## 10. 许可、依赖和发布

- 产品源码许可为闭源商业；`package.json` 为 `UNLICENSED`，Cargo workspace 为 proprietary internal。
- 可发行的开源依赖只允许经审计的 MIT、Apache-2.0、BSD 等宽松许可；清单在 `docs\compliance\F2S-SUPPLY-INVENTORY-001.json`。
- 运行 `node tools\compliance\license-inventory-check.mjs` 验证声明与锁文件；新依赖必须先更新审计材料，不能只看仓库首页许可徽章。
- WebView2 和 Spine 属于外部专有前置，不应被伪装成开源发行依赖。
- 代码签名、干净机验证、真实 Spine 往返、真实远程执行和组织 Release Gate 未完成时，候选不得公开发布。

卸载便携候选只需停止进程并移走应用包；**不要自动删除 `%LOCALAPPDATA%\FlashToSpine`**。项目数据、DPAPI 密钥和用户导出必须由用户在备份后显式处置。
