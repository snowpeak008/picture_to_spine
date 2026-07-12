# 代码模式与惯例

> 第一次阅读项目后填写。这里只记录目标项目已经采用的模式，不凭空发明规则。

## 文件组织

- Rust workspace 分为 `domain`、`application`、`adapters` 和桌面 delivery；依赖只从外向内。
- React UI 只通过 `native/ipc.ts` 的版本化合同访问 Rust 宿主。
- schema、docs、tests 与 evidence 是产品合同的一部分，不是实现后的可选补充。

## 命名规则

| 类型 | 规则 | 示例 |
|---|---|---|
| 文件 | Rust `snake_case.rs`；React 组件 `PascalCase.tsx` | `spine_cli_host.rs`、`AppShell.tsx` |
| 类/类型 | Rust/TypeScript `PascalCase` | `ProjectManifest`、`StyleSpec` |
| 函数/方法 | Rust/TypeScript `snake_case`/`camelCase`，表达动作和边界 | `navigate_to_string_and_verify`、`openProject` |
| 稳定代码 | 对外错误/状态使用 `F2S-*` 或明确枚举 | `F2S-BOOT-DOM`、`NOT_RUN_EXTERNAL` |

## 导入与依赖

- Cargo/npm 依赖固定并由 lock 文件约束；Core 构建使用 `--locked --offline`。
- 新发行依赖先做许可审计，只接受政策允许的宽松许可。
- Domain 不导入 I/O/GUI/适配器；UI 不导入通用网络、shell 或文件系统客户端。

## 错误处理

- 安全、路径、审批、版本和外部能力失败时 fail-closed，返回稳定且可操作的错误分类。
- 不吞掉启动错误；WebView 导航/DOM 失败必须明确报 `F2S-BOOT-NAVIGATION`/`F2S-BOOT-DOM`。
- `NOT_RUN`、`UNVERIFIED`、`EXTERNAL` 保持原义，不能换算成成功。

## 数据读写

- Rust 是持久化权威；revision/哈希/人工审批绑定后才允许推进。
- CAS 按内容哈希寻址；ProjectHead、signed sidecar、anchor 使用 DPAPI CurrentUser 包装密钥和 HMAC 校验。
- 文件提交采用 staging/不可变写入/原子发布；不手工覆盖现有导出或项目投影。

## 测试模式

- 领域不变量用 Rust unit/integration tests；跨层合同用 Node 静态/集成测试；外部能力必须另有真实 evidence。
- 边界测试包含 plus-one、篡改、重放、错误版本、错误路径、超时和 stale revision。
- WebView 本机探针是显式可选 GUI 测试，不放入 headless `npm test`。

## 提交前检查

```bash
npm run format:check
npm run lint
npm run typecheck
npm test
npm run test:integration
npm run test:spine
node tools/compliance/license-inventory-check.mjs
node tools/ci/verify-ci-contract.mjs
# 发布相关改动再运行：npm run package:core && npm run test:package
# 本机 GUI 允许时再运行：npm run test:webview-local
```
