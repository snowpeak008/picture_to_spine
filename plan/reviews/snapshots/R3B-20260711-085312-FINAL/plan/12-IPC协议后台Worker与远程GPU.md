---
doc_id: F2S-DOC-IPC-001
revision: 1.8
status: reviewed
canonical_for: [F2S-IFC-WORKER, F2S-IFC-RGPU, F2S-TST-IPC, F2S-TST-RGPU, F2S-GATE-IPC-SANDBOX-001]
depends_on: [F2S-DOC-GOV-001, F2S-DOC-REQ-001, F2S-DOC-ARCH-001, F2S-DOC-DOMAIN-001, F2S-DOC-STORE-001, F2S-DOC-SEC-001]
review_score_ref: F2S-SCORE-DOC-IPC-001-R3B
last_verified: 2026-07-11
---

# IPC 协议、后台 Worker 与私有远程 GPU

## 1. 目标和边界

本协议把 Rust 桌面核心与可选 AI Worker 隔离。Worker只执行受控计算，不拥有项目、审批、历史、导出或网络政策。没有Worker、GPU和网络时，项目管理、手工分层、Rig、动画、预览与导出仍可使用。

远程GPU仅指用户或其组织控制的本机、局域网或企业私有端点；第三方公共云、自动fallback和遥测不在架构中。所有网络由Rust `PrivateRemoteProvider`发起，WebView和Python本地Worker均无通用网络权限。

## 2. 端口与适配器

| ID | Port | Adapter | 责任 |
| --- | --- | --- | --- |
| F2S-IFC-WORKER-001 | ComputeProvider | ManualProvider | 纯手工路径和测试基线 |
| F2S-IFC-WORKER-002 | ComputeProvider | LocalOrtProvider | Rust进程内经批准ONNX小模型 |
| F2S-IFC-WORKER-003 | ComputeProvider | LocalPythonProvider | 受控子进程NDJSON协议 |
| F2S-IFC-RGPU-001 | ComputeProvider | PrivateRemoteProvider | 用户自托管HTTPS Job API |
| F2S-IFC-WORKER-004 | ArtifactGateway | ProjectArtifactAdapter | 只读输入和隔离输出提升 |

Application只依赖Port；Provider切换是Strategy。外部消息通过Anti-Corruption Layer转换为领域值，协议DTO不能直接成为AssetSpec/RigIR事实源。

## 3. 本地进程模型

Rust创建Worker子进程，设置隐藏窗口、固定工作目录、最小环境变量、受控stdin/stdout/stderr。不得通过PowerShell/cmd拼接命令。启动参数只包含协议版本、job工作根和只读模型manifest路径；密钥不通过命令行传递。

通信使用UTF-8、每行一个JSON对象的NDJSON：

```json
{"protocol":"f2s-worker","wire":{"major":1,"minor":0},"kind":"request","sessionId":"worker-session-01J...","id":"job-01J...","method":"segment","methodSchema":1,"idempotencyKey":"sha256:...","params":{"input":"in/source.png","outputDir":"out"}}
{"protocol":"f2s-worker","wire":{"major":1,"minor":0},"kind":"event","sessionId":"worker-session-01J...","id":"job-01J...","seq":1,"event":"stage","data":{"name":"decode"}}
{"protocol":"f2s-worker","wire":{"major":1,"minor":0},"kind":"event","sessionId":"worker-session-01J...","id":"job-01J...","seq":2,"event":"progress","data":{"completed":2,"total":10,"unit":"tile"}}
{"protocol":"f2s-worker","wire":{"major":1,"minor":0},"kind":"response","sessionId":"worker-session-01J...","id":"job-01J...","seq":3,"ok":true,"result":{"artifacts":[{"path":"out/mask.png","sha256":"...","mediaType":"image/png"}]}}
```

图片不得Base64进入消息。路径必须是job根目录下的规范化相对路径；Rust在发送前和接收后检查canonical path、symlink/reparse point、大小、媒体签名和SHA-256。

## 4. 协议 envelope

所有消息必须包含：`protocol`、`wire.major`、`wire.minor`、`kind`、`sessionId`、`id`。hello 的 `id=sessionId`；request/control/event/response 的 `id=JobId`。请求之后的 event/response/state_snapshot 还含严格递增 `seq`。单一整数 `v` 不再是 wire contract。未知 major 版本拒绝；相同 major 下，客户端与 Worker 协商双方都支持的 minor，只有 schema 明确标记可忽略的新字段才能向前兼容。未知 kind/method/error code 必须返回协议错误而不是崩溃。

### 4.1 固定消息资源上限

Wire v1 的解析上限是协议常量，不由不可信 Worker 提高：

| 常量 | v1 上限 | 超限行为 |
| --- | ---: | --- |
| `maxLineBytes` | 1 MiB（含换行） | 增量 reader 在分配更大 buffer 前终止请求并关闭 Worker |
| `maxJsonDepth` | 32 | 协议错误，当前执行 `failed` |
| `maxStringBytes` | 256 KiB | 协议错误；日志不得复制原字符串 |
| `maxArtifactsPerResponse` | 4096 | 响应隔离并失败 |
| `maxEventsPerSecond` | 100，短时 burst 200 | UI 合并 progress；持续超限先警告再终止 |
| `maxStdoutBytesPerJob` | 16 MiB | 终止并标协议资源错误 |
| `maxStderrRetainedBytes` | 4 MiB rolling | 截断并记录计数，不进入协议 |
| `partialLineTimeout` | 5 秒（running 且有 heartbeat 时可重置） | ping 后仍不完整则 `interrupted` |

JSON 使用限深流式解析；禁止先无界读取整行再检查长度。`params/details/state_snapshot` 仍须各自 schema 限制数组、字段和数值范围。所有边界值、上限±1、无换行长流、event flood 和深层 JSON 都进入 `F2S-TST-IPC-002`。

### F2S-IFC-WORKER-005 — capability handshake

Worker启动后的第一条有效响应必须声明：

```json
{
  "protocol":"f2s-worker",
  "wire":{"major":1,"minor":0},
  "kind":"hello",
  "sessionId":"worker-session-01J...",
  "id":"worker-session-01J...",
  "seq":0,
  "workerVersion":"0.1.0",
  "runtime":{"python":"3.12.x","device":"cuda","vramMiB":8192},
  "capabilities":[
    {"method":"segment","schema":1,"devices":["cpu","cuda"],"models":["manual","sam-small"]}
  ],
  "modelManifestHash":"sha256:...",
  "networkAccess":false
}
```

`hello.sessionId=hello.id` 是本次 Worker 进程唯一 `WorkerSessionId`，不是 JobId；`seq=0` 固定为握手起点。任何 request/event/response/control/state_snapshot 都必须携带相同 `sessionId` 和各自 JobId。hello 缺 `sessionId/id/seq`、二者不相等、重复 hello、hello 前出现其他 kind、未知 major 或协商失败都必须终止 Provider 握手，不能尝试猜测兼容。

桌面核心根据能力决定是否显示任务选项。版本或许可 manifest 不符时只禁用对应 Provider，不阻断手工核心。

## 5. Job状态和消息语义

Job 持久状态由 `F2S-DOC-DOMAIN-001` 定义为 `requested`、`queued`、`running`、`cancel_requested`、`cancelled`、`failed`、`succeeded`、`interrupted`。`validate`、`start`、`stage`、`progress`、`checkpoint` 是协议事件/投影，不新增持久状态；`waiting_for_approval` 是工作流门禁，不是 Worker Job 状态。

规则：

- `progress`只用于展示，不决定领域状态；
- Worker 不能发送 `candidate` 或 `approved`；只能返回待 Rust 校验的输出字节/descriptor 与终态候选；
- 同一 `seq` 重复事件可幂等忽略；跳号/乱序后必须停止应用增量事件并执行下述 resync；
- response 到达与 cancel 竞态由 Rust 仲裁，只持久一个终态；
- Worker退出但无response时为`interrupted`，由策略决定安全重试；
- checkpoint只在声明的阶段边界有效，不承诺任意算子中点恢复。

### F2S-IFC-WORKER-007 — resync

客户端发现期望 `seq=N+1` 但收到其他序号时，发送：

```json
{"protocol":"f2s-worker","wire":{"major":1,"minor":0},"kind":"control","sessionId":"worker-session-01J...","id":"job-01J...","control":"resync","afterSeq":12}
```

Worker 必须返回受限 schema 的 `state_snapshot`，包含相同 sessionId/JobId、当前 stage、严格递增 seq、最后已确认请求 hash、资源摘要、checkpoint descriptor hash 和 Worker 观察到的终态候选；它不得宣称领域 candidate/approved。可用时重放缺失事件。如 Worker 无法重建缺失序列，返回 `resync_unavailable`；Rust 将本次执行置为 `interrupted`，不猜测进度或终态。私有远程 SSE 使用同一 `seq` 语义和 `afterSeq` 恢复查询。

### 终态仲裁

Rust 调度器是终态唯一权威：

1. success 输出只有完成 schema/hash/安全/完整 GenerationProvenance 校验并作为不可变 `JobOutputArtifact` 提升到 CAS 后，Rust 才能持久 `succeeded`；若该终态先于 cancel command 持久，`succeeded` 胜出，后到 cancel 返回 already-terminal。
2. `cancel_requested` 若先持久，后到 success 只能进 quarantine，不得转为 `succeeded`；Provider 确认停止后转 `cancelled`。
3. 断线/崩溃导致顺序不可证明时终态为 `interrupted`，不伪造 cancelled/succeeded。
4. 终态一旦持久不再迁移；重试创建新 JobId。

### F2S-IFC-WORKER-006 — 取消

Rust先发送：

```json
{"protocol":"f2s-worker","wire":{"major":1,"minor":0},"kind":"control","sessionId":"worker-session-01J...","id":"job-01J...","control":"cancel","reason":"user"}
```

Worker应停止接收新工作、释放临时GPU资源并返回cancelled。宽限期后Rust终止进程树；未完成输出进入quarantine。取消不是错误，不自动重试。

## 6. 幂等、超时与重试

幂等键由 `method + input artifact hashes + normalized params + model/version/hash + protocol schema` 计算。只有输出尚未批准且hash完整时才能复用缓存。

| 场景 | 策略 |
| --- | --- |
| 启动超时 | 终止进程，记录capability失败，可人工重试 |
| 心跳/进度超时 | 先发送ping，仍无响应则interrupted |
| 可重试模型错误 | 保留输入，指数退避但最多由用户策略配置次数 |
| OOM | 不自动降分辨率/换模型；返回预算与建议，用户确认新任务 |
| 非法输出/hash错 | 隔离并terminal failure，不进入缓存 |
| 取消 | 不重试 |
| 相同幂等键并发 | 合并观察者或拒绝第二任务，不重复占GPU |

GPU默认并发1；CPU轻任务可按能力和内存预算并发。任务排队公平且可调整优先级，不允许前端直接绕过调度器。

## 7. Artifact提升协议

Worker 输出先进入项目外的 canonical sandbox：`%LOCALAPPDATA%/<Publisher>/FlashToSpine/job-sandboxes/<jobId>/out/`。禁止在 `.f2sproj` 内创建 `.jobs/`；Rust 执行：

1. 解析响应schema；
2. 约束路径在job root；
3. 校验大小、文件签名、像素上限、alpha和hash；
4. 运行安全/领域validator；
5. 由 Rust 补齐并验证内部 GenerationProvenance；
6. 复制或原子提升为内容寻址 `JobOutputArtifact`；
7. 持久化 Job `succeeded`，此时输出仍为 `unbound`，没有 Candidate revision；
8. 用户/用例另行提交带 `expected_revision`、output hash、target ID 和幂等键的 `Register*Candidate` 事务；
9. Candidate 经人工批准后才更新批准引用。

Worker不得写 `project.json`、审批日志、RigIR或exports目录。失败/未知文件隔离，清理需要独立可恢复命令。

内部本地/私有远程任务的 GenerationProvenance 必须由 Rust 绑定：Provider/Worker ID、wire/API 版本、模型/算法 ID 与精确版本、runtime/model manifest hash、规范化参数、seed（适用时）、设备/执行位置、JobId、idempotency key、输入/遮罩/Spec revision 与 hash、输出 hash、开始/结束时间、重试/取消/恢复父链和所有质量降级。Worker 提供的同名字段只是不可信候选；任一必填项无法由 Rust/已审计 manifest 证明时输出进 quarantine，不能 `succeeded`。

`unknown_external` 只适用于用户从产品外部回导的文件，不得用于放宽本协议内部 Provider 的 provenance 硬门。

## 8. 错误契约

错误响应：

```json
{
  "protocol":"f2s-worker","wire":{"major":1,"minor":0},"kind":"response",
  "sessionId":"worker-session-01J...","id":"job-01J...","seq":3,"ok":false,
  "error":{"code":"F2S-WORKER-RESOURCE-001","retryable":false,"messageKey":"worker.oom","details":{"requiredVramMiB":11264,"availableVramMiB":7168}}
}
```

错误码以F2S-DOC-ENG-001的统一registry为准。`details`有大小上限并禁止绝对路径、token、prompt全文和图像字节。未知错误映射为稳定的`F2S-WORKER-UNKNOWN-001`，原始堆栈只进入本地脱敏debug日志。

## 9. 私有远程 GPU API

### 9.1 端点

建议版本化API：

- `GET /v1/capabilities`；
- `POST /v1/jobs`，仅发送已审批upload manifest；
- `GET /v1/jobs/{id}`；
- `GET /v1/jobs/{id}/events`，SSE为基线；
- `POST /v1/jobs/{id}:cancel`；
- `PUT/GET /v1/artifacts/{id}`，使用hash和大小校验；
- `DELETE /v1/jobs/{id}`，按服务保留政策请求清理。

协议必须做 capability/schema 握手，不假定远端与桌面版本相同。远端成功响应仍只能形成经 Rust 校验并持久的未绑定 `JobOutputArtifact`；必须经过独立 `Register*Candidate` 事务才成为 candidate。

### 9.2 远端 Job DTO、幂等与授权

`POST /v1/jobs` 的 v1 请求至少为：

```json
{
  "apiVersion":"f2s-rgpu/v1",
  "requestId":"01J...",
  "idempotencyKey":"sha256:...",
  "method":"segment",
  "methodSchema":1,
  "uploadManifestHash":"sha256:...",
  "inputArtifacts":[
    {"artifactId":"sha256:...","sha256":"...","size":1234,"mediaType":"image/png"}
  ],
  "normalizedParamsHash":"sha256:...",
  "model":{"id":"...","version":"...","manifestHash":"sha256:..."},
  "retention":{"deleteAfterSeconds":3600,"requireDeleteReceipt":true}
}
```

上传批准记录必须绑定 endpoint profile ID、证书/组织身份、`uploadManifestHash`、对象列表/大小、method/model、保留政策和当前项目 revision。Rust 在请求前重算 manifest hash；UI 批准后任何字段变化都必须重新批准。

授权使用 `Authorization: Bearer` 短期 token 或 mTLS，令牌至少限制 endpoint audience、method、最大字节、过期时间；只保存在 Windows Credential Manager，不进入 body、URL、项目或日志。服务把同一 `idempotencyKey + request body hash` 返回同一 JobId；相同 key 不同 body 必须 `409 idempotency_conflict`，不得重复计算。

响应必须包含 `jobId`、`requestHash`、`acceptedManifestHash`、`serverCapabilityHash`、`retentionDeadline` 和事件序列起点。下载 Artifact 的 hash/大小/mediaType 必须与完成清单一致。`DELETE` 成功返回版本化 `DeletionReceipt`（jobId、artifact hashes、requestedAt、deletedAt、status、server identity/signature）；客户端保存 receipt hash，但仍把远端物理删除声明标为外部证据，不宣传为本地可证明事实。超时/不支持 receipt 时持续显示残余离机风险。

### 9.3 信任和网络政策

- Provider默认关闭；配置时显示主机、解析IP、证书和组织标签；
- 所有产品构建都拒绝明文HTTP、跳过证书验证和无认证端点；隔离测试的localhost证书也必须显式加入临时测试信任库，不能关闭验证；
- 支持TLS、证书指纹和可选mTLS；短期token保存Windows Credential Manager；
- DNS/IP变化、证书变化或目标从私网变公网必须重新审批；
- 发送前展示每个artifact、分类、大小、hash、目标和服务声明保留期；
- 不实现任意第三方URL、自动发现公共Provider或失败转云；
- 前端没有HTTP权限，所有请求经Rust policy enforcement。

### 9.4 Worker网络隔离

`F2S-NFR-SEC-005` 的商业 D2 本地 Worker 唯一隔离 Profile 固定为 `windows-appcontainer-v1`，不存在“受限 token 或 ACL”等二选一发布路径。该 Profile 的全部控制同时必需：

1. 使用 restricted AppContainer token 启动 Worker，capability 列表不授予 network capability；
2. Job root 使用该 AppContainer SID 专用 ACL：`in` 只读、`out/tmp` 仅该 Job 可写，项目根、其他 sandbox、用户目录和凭据库不可达。签名Runtime Pack中确切列入manifest的worker/runtime/DLL可获该profile SID最小`RX`且禁止写；内置模型只读，用户模型必须经hash/schema验证后复制到本Job的`in/model`并设只读，不能给AppContainer开放原模型目录。父目录遍历权只授予到allowlisted叶节点，邻接文件保持拒绝；
3. Worker 及全部后代加入同一个 Windows Job Object，启用 kill-on-job-close、进程/内存/CPU 上限，禁止逃逸子进程；
4. Rust 不向 Worker 传递 token、代理、用户环境或通用文件句柄；
5. 运行 OS egress 恶意探针，覆盖 TCP/UDP/DNS/loopback/子进程代理尝试，并由外部网络观测确认零外连；
6. 路径、junction/reparse point、句柄继承和子进程逃逸恶意 fixture 全部被 OS 边界拒绝。

`F2S-GATE-IPC-SANDBOX-001` 是启用 LocalPythonProvider/发布 Worker Pack 的硬门：M00 Spike 必须在目标 Windows 11 机器证明 Profile 可创建、正常任务可运行、六类控制都可测，并产生 `F2S-EVD-M00-004`；M09 再以签名候选包执行恶意回归。任一控制不可用、证据缺失或回归失败时，LocalPythonProvider 与 Worker Pack 从能力层禁用，不显示可执行选项，不允许 `policy_only` 处理 D2。手工核心和经审计的 Rust 进程内能力继续运行。

`F2S-TST-RGPU-005` 直接验证 `F2S-NFR-SEC-005` 与本 Gate，不接受应用约定、环境变量、Worker 自报 `networkAccess=false` 或单独防火墙规则作为替代证据。

## 10. 隐私与日志

协议日志记录job ID、方法、阶段、耗时、资源摘要、错误码和hash前缀，不记录图片、mask、完整prompt、绝对路径、凭据或HTTP body。远端诊断默认只保存在本机。用户生成诊断包前看到清单；没有自动上传。

本地文件删除不能保证SSD物理擦除；项目应依赖用户磁盘加密策略（如BitLocker）保护静态数据，并把这一残余风险写入安全文档。

## 11. 兼容与迁移

- `wire.major` 不兼容时拒绝启动任务；
- `wire.minor` 通过能力协商，每个可选字段/方法必须在 capability 中声明；
- Worker/runtime/model manifest分别版本化；
- 正在运行的旧协议任务不在升级中强行迁移，先取消或完成；
- 升级失败时桌面核心回退到ManualProvider，不回退项目schema；
- 私有远端API升级须同时支持前一个minor过渡期，删除能力前给deprecation日期。

## 12. 测试矩阵

| Test ID | 场景 | 期望 |
| --- | --- | --- |
| F2S-TST-IPC-001 | hello/capability正常与异常 | 正常 session 映射精确；缺/错 sessionId/id/seq、重复/前置 event、未知 major 均 fail closed |
| F2S-TST-IPC-002 | malformed/资源上限/未知major | 对每个固定阈值测试上限±1、无换行流、深层JSON和event flood；限长增量读取，桌面不崩溃 |
| F2S-TST-IPC-003 | seq重复/跳号/乱序 | 重复幂等；跳号/乱序执行 resync，不可重建时转 interrupted，不重复提交状态 |
| F2S-TST-IPC-004 | cancel、success与注册竞态 | 只有一个 Job 终态；succeeded 只产生未绑定 JobOutputArtifact，RegisterCandidate 另事务且 revision 冲突可重试 |
| F2S-TST-IPC-005 | Worker崩溃/进程树残留 | interrupted，可安全重启，无孤儿GPU进程 |
| F2S-TST-IPC-006 | `../`、绝对路径、junction/symlink | 路径逃逸全部拒绝 |
| F2S-TST-IPC-007 | hash、媒体签名、尺寸不符 | artifact quarantine，不进入项目 |
| F2S-TST-IPC-008 | OOM | 不静默降级，返回预算和新任务建议 |
| F2S-TST-RGPU-001 | 未启用远端 | 网络观测无项目数据外发 |
| F2S-TST-RGPU-002 | 公网/HTTP/证书变化 | 默认拒绝或要求企业策略重新批准 |
| F2S-TST-RGPU-003 | TLS中断/断线/重复提交 | request/upload manifest hash 绑定；同key同body复用、同key异body 409；安全重试和确定状态 |
| F2S-TST-RGPU-004 | 返回/保留契约 | hash错/非法schema隔离失败；auth scope/expiry生效；DeletionReceipt与请求对象匹配，缺receipt持续显示外部残余风险 |
| F2S-TST-RGPU-005 | `F2S-NFR-SEC-005` / `windows-appcontainer-v1` | 合法签名worker可从manifest allowlist的RX runtime/DLL和本job只读模型启动；相邻未列文件、原模型目录、项目/其他job均拒绝；另验证无network capability、Job Object及OS egress/路径/子进程探针，任一失败则Provider/Pack禁用 |
| F2S-TST-RGPU-006 | 日志/诊断包检查 | 无敏感body、token、路径或图像数据 |

## 13. 非目标

- 不支持第三方公共云Provider；
- 不在Worker中保存项目、审批或导出状态；
- 不让WebView拥有通用网络、shell或文件系统权限；
- 不承诺跨任意Python环境兼容；Worker使用经批准的独立Runtime Pack；
- 不通过协议传输Spine激活信息；
- 不把远程服务的“成功”自动等同于内容合格或已批准。

## 14. 完成条件

协议schema、Rust/Python生成类型、mock Worker和私有远端mock通过全部契约与故障测试；核心功能在Worker缺失/崩溃/无网络时仍可用；未知路径、许可或网络目标fail closed；真实远端在没有用户端点时保持`EXTERNAL_UNVERIFIED`。
