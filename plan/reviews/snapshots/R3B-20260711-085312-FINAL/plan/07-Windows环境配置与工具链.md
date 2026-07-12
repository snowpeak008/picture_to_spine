---
doc_id: F2S-DOC-ENV-001
revision: 1.6
status: reviewed
canonical_for:
  - F2S-IFC-005
  - F2S-TST-070
  - F2S-TST-071
  - F2S-TST-072
  - F2S-TST-073
  - F2S-TST-074
  - F2S-TST-075
  - F2S-TST-076
  - F2S-TST-077
  - F2S-TST-078
  - F2S-TST-079
  - F2S-ADR-ENV-001
  - F2S-ADR-ENV-002
  - F2S-ADR-ENV-003
  - F2S-ADR-ENV-004
  - F2S-ADR-ENV-005
depends_on:
  - F2S-DOC-GOV-001
  - F2S-DOC-REQ-001
  - F2S-DOC-ARCH-001
review_score_ref: F2S-SCORE-DOC-ENV-001-R3B
last_verified: 2026-07-11
---

# Windows 环境配置与工具链

## 1. 目标与原则

本文档规定开发、测试、打包和用户运行环境。环境计划必须同时满足：

- Windows 桌面应用可以从主目录的单一入口文件双击启动；
- 没有 Python、没有 GPU、没有 Spine 时，桌面核心仍能打开项目并使用手工工作流；
- Python、CUDA 和模型不污染核心安装包；
- 开发环境和发布环境有精确版本、锁文件和可生成的许可证据；
- 任何降级都由能力探测和用户确认驱动，不依赖未捕获异常。

依 `F2S-ADR-REGISTRY-001`，`F2S-ADR-ENV-001`至`005`的implementation-local canonical body由本文件拥有；24号只登记其exact owner和状态。

## 2. 平台支持矩阵

| 平台 | 等级 | 验收范围 |
| --- | --- | --- |
| Windows 11 x64 23H2/24H2 及后续受支持版本 | P0 一级 | 开发、CI、安装、GPU、高 DPI、中文路径和 Spine CLI 全量验收 |
| Windows 10 x64 22H2 | P1 best-effort | 核心安装与手工工作流；不作长期 OS 安全支持承诺，不阻塞 P0 发布 |
| Windows on ARM | 非首版 | 不阻止 schema 和工程的架构兼容，不生成发布包 |
| macOS/Linux/Web | 非目标 | 不进入验收矩阵 |

文件系统验收必须包含 NTFS、长路径、中文/日文/空格/表情符号路径、只读目录、磁盘将满、防病毒占用文件和网络盘失联。生产项目不建议直接放在云盘同步目录；如用户坚持，应显示并发写入风险提示。

## 3. 工具链决策

| 决策 ID | 内容 | 证据位置 |
| --- | --- | --- |
| `F2S-ADR-ENV-001` | 核心前端使用 Node.js 24 LTS 系列、npm 11 系列和 npm workspaces，但仓库在首次环境 Spike 后锁定精确 patch | `.node-version`、`packageManager`、`package-lock.json` |
| `F2S-ADR-ENV-002` | Rust 使用 `x86_64-pc-windows-msvc` 稳定工具链，精确版本写入 `rust-toolchain.toml` | `rust-toolchain.toml`、`Cargo.lock` |
| `F2S-ADR-ENV-003` | 可选 Worker 使用 Python 3.12.x 和 `uv.lock`，不使用用户系统 Python | `workers/ai-worker/.python-version`、`uv.lock` |
| `F2S-ADR-ENV-004` | 核心桌面包和 AI Runtime Pack 独立构建、签名、安装、更新和生成 SBOM | `dist/manifests/*.json` |
| `F2S-ADR-ENV-005` | Windows 安装器使用 Tauri NSIS per-user，压缩算法强制为 Zlib，使用系统 Evergreen WebView2 | `tauri.conf.json`、打包报告 |

Node、Rust、npm、Python 的上述系列只是开发候选基线。`F2S-DEV-M00-001` 环境与决策 Spike 必须在清洁 Windows 11 VM 上输出精确 patch 及 `F2S-EVD-M00-001` 证据，然后写入 `.node-version`、`packageManager`、`rust-toolchain.toml`、`.python-version` 和锁文件。M00 之前所有工具版本状态为 `UNVERIFIED`；不允许锁文件中出现 `latest`、通配版本或浮动 Git branch。

## 4. 开发机前置

### 4.1 必选前置

| 工具 | 用途 | 安装/验证要求 |
| --- | --- | --- |
| Git | 源码和版本管理 | 验证 longpaths 配置，不把凭据写入仓库 |
| Visual Studio 2022 Build Tools | MSVC 链接、Windows SDK | 安装 Desktop development with C++、MSVC v143、Windows 11 SDK |
| Rustup/Rust/Cargo | Rust 核心构建 | 必须遵循 `rust-toolchain.toml` |
| Node.js + npm | React/Vite 构建、JS 依赖和 workspaces | npm 由 Node 工具链提供，精确版本由 `packageManager` 字段固定；不全局安装项目 CLI |
| WebView2 Runtime | Tauri UI 运行 | 优先使用操作系统 Evergreen Runtime |

### 4.2 可选前置

| 工具 | 启用功能 | 失缺时行为 |
| --- | --- | --- |
| Python 3.12 Runtime Pack | 本地 Python 模型 | 显示“本地高级 AI 不可用”，手工/轻量策略可用 |
| NVIDIA 驱动/合容 GPU | CUDA 推理 | 不安装驱动，切换 CPU/DirectML 候选或私有远程 |
| Spine Professional/Enterprise 4.2.43 | 官方 CLI 往返验证 | 仍可输出 Rig IR/PSD/PNG/候选 JSON，状态显示“未执行官方验证” |
| 用户自托管私有 GPU | 高成本分层/补全 | 远程 Provider 不出现在可执行选项中 |

应用不得读取、储存或代理 Spine 激活码。它只记录用户配置的 `Spine.com` 路径、探测版本、执行结果和时间。

## 5. 硬件能力档位

| 档位 | 建议硬件 | 计划能力（非承诺） | 明确边界 | 证据状态 |
| --- | --- | --- | --- | --- |
| H0 核心档 | 4核 CPU、16GB RAM、10GB 可用磁盘 | 项目、手工分层、Rig、时间轴、导出 | 不包含本地生成式补全 | `UNVERIFIED`，待 M00/目标夹具证据 |
| H1 8GB GPU 档 | RTX 3060 Ti 8GB 等级、32GB RAM、50GB 可用磁盘 | SAM 2 tiny/small 类轻量交互分割、代理图和分块推理 | 不承诺 Qwen-Image-Layered 原精度实时推理 | `UNVERIFIED`，待 `F2S-DEV-M00-004` 生成 `F2S-EVD-M00-004` |
| H2 本地高档 | 16–24GB+ VRAM、64GB RAM | 通过未来基准的高成本模型 | 未经 manifest 审计和基准测试的任意模型 | `UNVERIFIED`，无证据不显示为 available |
| H3 私有远程档 | 用户自托管 GPU，TLS 端点 | 服务端 manifest 声明且客户端验证的能力 | 第三方 SaaS、隐式上传或无删除承诺端点 | 每端点初始 `UNVERIFIED`，握手/微型夹具后产生端点证据 |

应用首启动和配置变更时生成 `CapabilityReport`，包含 CPU/RAM、GPU 名称、驱动、可用 VRAM、推理后端、模型 manifest、Spine 版本、磁盘空间和每项能力的 `UNVERIFIED|VERIFIED|FAILED` 证据状态。证据必须引用由对应原子任务声明的精确 `F2S-EVD-<milestone>-<sequence>` ID；没有精确证据 ID 不得从“计划能力”升格为已验证，报告也不得自动上传。

## 6. 环境探测接口 `F2S-IFC-005`

```rust
struct CapabilityReport {
    report_version: u32,
    os: OsCapability,
    cpu: CpuCapability,
    memory: MemoryCapability,
    gpu: Vec<GpuCapability>,
    local_workers: Vec<WorkerCapability>,
    spine: SpineCliCapability,
    storage: StorageCapability,
    remote_profiles: Vec<RemoteCapabilitySummary>,
    warnings: Vec<CapabilityWarning>,
}

trait EnvironmentProbe {
    fn inspect(&self) -> Result<CapabilityReport, EnvironmentError>;
    fn test_worker(&self, worker: WorkerId) -> Result<ProbeEvidence, EnvironmentError>;
    fn test_spine_cli(&self, executable: &Path) -> Result<ProbeEvidence, EnvironmentError>;
    fn test_private_remote(&self, profile: RemoteProfileId)
        -> Result<ProbeEvidence, EnvironmentError>;
}
```

`test_private_remote` 只发送无用户图片的协议握手和微型自制测试数据。不得为了“测试连接”自动上传当前项目素材。

## 7. 本地开发模式

### 7.1 命令约定

下列名称是计划冻结的顶层工程接口；在 `package.json`、锁文件和脚本尚未由 `F2S-DEV-M00-001` 创建并通过 `F2S-EVD-M00-001` 实跑前，状态统一为 `planned`，不能描述为“当前可执行”或 `VERIFIED`。M00 通过后，脚本内部实现可以演进，但顶层命令名变更必须同步开发文档、CI 和证据。

| 命令 | 作用 | 前置 |
| --- | --- | --- |
| `npm run bootstrap:check` | 只读检查工具链和锁文件 | 无网络写入 |
| `npm run dev` | 启动 Tauri + Vite 开发环境 | 不自动启动 Python Worker |
| `npm run dev:ai` | 在已安装可选运行包时附加本地 Worker | 先运行许可和能力检查 |
| `npm test` | JS/Rust/Python 可用部分的快速测试 | 不需要 Spine |
| `npm run test:integration` | Worker/存储/IPC 集成 | 使用假 Worker 和自制夹具 |
| `npm run test:spine` | 4.2.43 CLI 往返 | 需用户/构建机合法 Spine Professional/Enterprise |
| `npm run build:core` | 构建桌面核心 | 不包含 AI Runtime Pack |
| `npm run build:ai-pack` | 构建经批准的可选 Worker | 单独 SBOM/许可门禁 |
| `npm run release:verify` | 清洁包验证、签名前检查 | 锁文件不可变 |

这些命令是稳定工程界面。内部使用的 Cargo、Vite、uv 参数可变，但 CI 和开发文档只引用上述顶层命令。

### 7.2 配置与凭据

- 非秘密开发配置使用已提交的 `.env.example`，真实 `.env.local` 必须被忽略。
- 产品运行时不依赖工作目录 `.env`；非敏感配置进 `%LOCALAPPDATA%`，凭据进 Windows Credential Manager。
- 日志级别、模型路径和临时目录可用受控环境变量覆盖，但 UI 必须显示最终解析值。
- 任何加密私钥、代码签名凭据和远程 Token 不进入日志、崩溃包或项目导出。

## 8. AI Runtime Pack

Runtime Pack 的目录与核心应用分离，有自己的：

```text
runtime-pack/
  worker.exe
  runtime-manifest.json
  model-manifests/
  licenses/
  sbom/
  runtimes/                  # 经批准的 CPython/本地库
  models/                    # 可再拆成独立模型包
```

构建顺序：

1. `uv sync --frozen` 从锁文件创建隔离环境；
2. 检查直接和传递依赖许可；
3. 核对每个模型的代码、权重、数据声明和 SHA-256；
4. 执行fail-closed Python packaging Spike：候选编译器/冻结器本身、构建插件、输出Runtime和全部传递依赖都按`F2S-LIC-POLICY-001`锁版审计；当前不预选任何工具；
5. 在无 Python 的干净 Windows VM 运行 smoke test；
6. 生成独立 SBOM、THIRD_PARTY_NOTICES 和签名 manifest；
7. 只有该包通过门禁后，桌面核心才能把它标为 trusted Worker。

Python 自身是 PSF-2.0，CUDA/cuDNN 受 NVIDIA 条款约束。这些不能由 PyTorch 的 BSD-style 许可替代。Runtime Pack 的发布是单独的合规决策，不是桌面核心通过即自动通过。

## 9. 打包、安装与双击入口

### 9.1 发布产物

```text
dist/
  FlashToSpine-<version>-x64-setup.exe
  FlashToSpine-portable-x64/
    FlashToSpine.exe
    THIRD_PARTY_NOTICES.html
    manifests/
  checksums.txt
  sbom/
  signatures/
```

NSIS 设置：

- per-user 安装到 `%LOCALAPPDATA%\Programs\<Publisher>\FlashToSpine`，默认不请求管理员权限；
- `compression: "zlib"`，不使用默认 LZMA；
- 不捆绑 Fixed WebView2，安装前检查 Evergreen Runtime；
- 不捆绑 Python、CUDA、Spine Editor、Spine Runtime 或模型权重；
- 安装、卸载和覆盖安装都必须在 Worker 运行时有明确关闭协议。

### 9.2 仓库主目录入口

入口文件名和行为只引用 `F2S-DOC-RELEASE-001` 的 `F2S-INSTALL-ENTRY-001`：项目完成时主目录必须存在 `FlashToSpine.cmd`。环境文档不另设中文别名或第二入口。它的唯一职责是：

1. 基于 `%~dp0` 定位仓库，正确处理空格和中文路径；
2. 优先启动 `dist\FlashToSpine-portable-x64\FlashToSpine.exe`；
3. 如可执行文件不存在，显示“尚未构建”和非技术用户可理解的下一步；
4. 返回应用退出码。

该入口不得自动安装依赖、下载模型、更改 PowerShell ExecutionPolicy、请求管理员权限或启动开发服务器。开发者入口另行使用 `npm run dev`。

### 9.3 WebView2 缺失策略

系统 Evergreen WebView2 是 Core 的运行前置，但缺失不能表现为白屏或一闪而过：

1. NSIS 在提交安装前检测 WebView2；缺失时保持安装器可见，显示稳定错误 `F2S-APP-WEBVIEW2-001`、Microsoft 官方离线/企业部署说明和取消选项。
2. `FlashToSpine.cmd`/发布启动链在主窗口创建失败时保持可见诊断，指向本地日志；不得自行联网下载 bootstrapper。
3. 完全离线介质默认不捆绑未经单独许可、哈希和签名审计的 WebView2；企业可预装 Microsoft 官方离线 Runtime。
4. 缺失 WebView2 不修改项目、Runtime Pack、Spine 或用户设置；安装前置补齐后可重新启动。
5. 安装包、启动器和文档对该前置使用同一检测条件，避免安装通过但应用白屏。

## 10. CI 与可重复构建

CI 使用 Windows x64 runner，至少分为：

1. `lint-and-unit`：格式、静态分析、单元测试、schema 代码生成差异；
2. `integration-core`：项目存储、假 Worker、IPC、恢复；
3. `license-and-sbom`：Cargo/npm/Python/模型清单审计；
4. `build-core`：冻结锁文件构建 portable 与 NSIS；
5. `clean-vm-smoke`：安装、双击启动、卸载、无 Python/GPU/Spine 运行；
6. `optional-ai-pack`：独立 Worker 包和各硬件档位基准；
7. `optional-spine-4.2.43`：只在有合法授权的受控 runner 执行，不将激活材料放入 CI 日志。

释放构建必须使用 `--frozen`/`--locked`。如锁文件在构建后变化，即使二进制成功也必须失败。

## 11. 失败处理

| 失败 | 用户可见结果 | 自动恢复边界 |
| --- | --- | --- |
| WebView2 缺失 | 安装器或启动器给出官方安装指引 | 未获授权不自动下载 |
| MSVC/Node/Rust 版本不符 | `bootstrap:check` 列出期望值和实际值 | 不自动升降级全局工具 |
| Worker Runtime 缺失/损坏 | Worker 标为 unavailable/untrusted，核心继续运行 | 可重新安装独立 Runtime Pack |
| GPU OOM | Job 失败且提供代理分辨率、tile、CPU 或私有远程选项 | 不静默修改质量参数 |
| Spine 不是 4.2.43 | 禁止官方验证任务，显示精确版本差异 | 不执行 `--update latest` |
| 安装包未签名 | 内部构建标识明确；商业发布门禁失败 | 不跳过 SmartScreen/签名检查 |
| 磁盘空间不足 | 构建/运行前阻止，告知预估需求 | 不自动删除项目产物 |

## 12. 环境测试

| 测试 ID | 场景 | 通过条件 |
| --- | --- | --- |
| `F2S-TST-070` | 干净 Windows 11 安装 | 安装后可双击启动，不需要 Python/GPU/Spine |
| `F2S-TST-071` | 主目录启动器 | 中文、空格和超过 100 字符路径可启动正确 EXE |
| `F2S-TST-072` | 锁文件构建 | 断网缓存完整时可重建，产物清单可比较 |
| `F2S-TST-073` | 8GB GPU 能力探测 | 正确归类 H1，不将未实测大模型标为可用 |
| `F2S-TST-074` | 无GPU/驱动异常 | 手工流程可用，没有启动崩溃 |
| `F2S-TST-075` | Spine 版本探测 | 只接受 4.2.43 执行官方往返，不保存激活信息 |
| `F2S-TST-076` | NSIS 审计 | 配置为 Zlib，包内不含 Spine Runtime、Python、CUDA 或模型 |
| `F2S-TST-077` | Runtime Pack 隔离 | 安装/卸载/损坏 Runtime Pack 不会导致项目不可打开 |
| `F2S-TST-078` | 许可拒绝 | 任意未知、NC 或 Research-only 运行依赖导致发布任务失败 |
| `F2S-TST-079` | WebView2 缺失 | 干净机无 Evergreen Runtime 时安装/启动保持可见诊断、返回 `F2S-APP-WEBVIEW2-001`、不联网下载且不修改项目；补齐前置后可启动 |

## 13. 许可注意事项

- Tauri 为 MIT/Apache-2.0，React/Vite/PixiJS 为 MIT，但实际锁文件中的传递依赖仍需逐包审计。
- CPython 是 PSF-2.0，已列入`F2S-LIC-POLICY-001`的可审计allowlist；Runtime Pack仍必须单独通过SBOM、传递依赖和模型许可门，不能因CPython本身通过而整体通过。
- 当前不设默认Python打包器。Nuitka官方仓库当前为AGPL-3.0，其runtime exception只涉及编译输出而不把compiler改成宽松许可；PyInstaller也不是默认。任一候选只有在锁定版本的工具许可、输出例外、插件、传递依赖和生成物全部通过`F2S-LIC-POLICY-001`后才可写入新决策，否则Worker Pack物理移除、Core手工链继续。
- NSIS 主体和 Zlib 压缩模块是宽松许可；默认 LZMA 模块是 CPL，因此必须显式更改压缩算法并检查生成物。
- WebView2、NVIDIA 驱动/CUDA 和 Spine Editor 是专有外部前置，必须在 THIRD_PARTY_NOTICES/用户条款中分类说明，不声称其为 MIT 依赖。

## 14. 核验依据

- Tauri Windows 安装和 WebView2 选项：<https://v2.tauri.app/distribute/windows-installer/>
- Tauri NSIS 默认 LZMA 及 Zlib 配置：<https://v2.tauri.app/reference/config/>
- NSIS 模块许可：<https://nsis.sourceforge.io/Docs/AppendixI.html>
- CPython 许可：<https://docs.python.org/3/license.html>
- Nuitka compiler许可与runtime exception核验源：<https://github.com/Nuitka/Nuitka>、<https://github.com/Nuitka/Nuitka/blob/develop/LICENSE-RUNTIME.txt>
- Spine 精确版本和 CLI：<https://esotericsoftware.com/spine-changelog> 与 <https://esotericsoftware.com/spine-command-line-interface>
