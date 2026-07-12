# Schema 与迁移维护说明

## 1. 权威来源

JSON Schema 的可编辑源位于 `schemas\src`。当前主要合同包括：

- `project.schema.json`：ProjectManifest，当前 `schemaVersion` 精确为 `1.4.0`；
- `storage.schema.json`：ProjectHead/存储 wire，当前 schema 为 `1.0.0`；
- `rig-candidate.schema.json`、`rig-ir.schema.json`；
- `motion-content.schema.json`、`animation-set.schema.json`；
- `ipc.schema.json`、`job.schema.json`、`evidence.schema.json`；
- `remote-gpu.schema.json`；
- `common.schema.json`、`governance.schema.json`。

`schemas\generated` 是生成物，不是手工权威源。Rust 领域类型、TypeScript IPC union、Schema enum 和 golden fixture 必须保持同一字段名、大小写、动作顺序与状态集合。

## 2. 再生与检查

修改 `schemas\src\*.json` 后，从项目根执行：

```powershell
node schemas/generate.mjs
npm run typecheck
npm test
npm run test:integration
```

生成器会：

- 解析并按文件名排序所有源 schema；
- 对规范化 JSON 计算 SHA-256；
- 重建 `schemas\generated\ts\index.ts`；
- 重建 `schemas\generated\rust\mod.rs`；
- 重建 canonical 和 adversarial golden JSON。

生成的 ActionKey 必须精确为：

```text
idle, run, jump, fall, dash,
attack_01, attack_02, attack_03,
hit, death
```

时间基固定为 `1/30000`。禁止把 `attack_01` 改写成别名、接受未知顶层字段、宽松解析大小写哈希，或用浮点时间代替整数 tick。

## 3. 版本原则

- schema 版本和产品版本独立。
- patch 表示不改变 wire 语义的修正；新增/删除/改义字段必须评估 minor/major。
- `additionalProperties: false` 的合同不允许消费者猜测未知字段。
- 读取未知未来版本必须 fail-closed；不允许“尽量打开”。
- 降级迁移禁止；不能把 4.2.43 能力结论外推到其他 Spine patch。
- 每次 schema 变更必须同时更新领域验证、IPC 两端、fixtures、测试和 schema registry hash。

## 4. ProjectHead 安全封装

生产 ProjectStore 使用 DPAPI CurrentUser 管理的 256-bit 密钥，并在提交时由 store 添加：

- 稳定 key ID；
- 当前 head HMAC-SHA256；
- 前一 manifest SHA-256 和前一 head MAC；
- 每 revision 的不可变 signed sidecar；
- 每项目最高 revision 的 HMAC anchor。

ProjectHead 的调用方不能自行提交 seal。打开项目时必须先验证 head、anchor 和完整 revision 链，再读取 CAS manifest。这一封装是本地完整性边界；它不把审批升级为自然人签名。

安全提交和恢复采用 authenticated roll-forward：

1. store 先写 immutable revision 数据与 sealed signed sidecar；
2. 再把 HMAC anchor 发布到该 revision；
3. 最后发布 `head.json` 投影；
4. 若进程在投影发布之间退出，读取端只接受 HMAC 有效、revision 连续且 `previousHeadMac`/manifest 前驱精确匹配的 signed sidecar，并把旧投影向前推进；
5. 若存在连续 signed successor，可以逐 revision 前进，但不能跳过缺口或选择较旧 successor。

genesis sidecar 是唯一提交事实且不存在后续 revision 时，缺失的 genesis head/anchor 可以从该 sidecar 重建。head-ahead 或 anchor-ahead 只允许一个 revision 的受验证中断恢复；相差更多、sidecar 篡改、错误前驱、分叉或高水位删除仍必须 fail-closed。这个机制是同一 schema 下的提交恢复，不是 schema migration，也不会让 unsigned legacy 项目变成已签名项目。

维护 `storage.schema.json` 时，必须和 Rust `ProjectHead` 的实际序列化字段保持一致。任何新增安全字段都要进入 schema、golden、tamper/rollback/fork 负向测试；不能通过将字段标为可选而让生产 store接受 unsigned head。

## 5. 当前没有自动迁移

生产 UI/CLI 没有提供自动迁移命令，也没有“首次打开自动签名旧项目”行为。旧 unsigned/unanchored ProjectHead 会在读取 manifest 之前被拒绝。authenticated roll-forward 只消费生产 store 已写出的有效 signed sidecar，不是 legacy migration 的替代路径。

源码中的 `open_project` 存在把已解析的 `1.1.0`、`1.2.0`、`1.3.0` 标记为 `1.4.0` 的内存兼容分支；它没有创建备份、迁移报告、新 CAS 对象、signed sidecar 或人工确认，因而**不是受支持的持久迁移流程**，不能在产品文档中称为自动迁移。安全生产 store 也不会借此接受 unsigned head。

`migrate_copy_on_write` 和 `migrate_copy` 目前只是低层原语：它们没有形成已接入宿主、逐版本验证、可恢复且可审计的项目迁移工具。测试或维护代码能调用这些原语，不代表用户项目获得迁移支持。

## 6. 未来显式迁移的最低合同

若后续实现迁移，必须作为独立受审功能，至少满足：

1. 用户明确选择源项目和目标版本，并在原生界面确认；
2. 在读取内容前验证源 ProjectHead/anchor/chain；unsigned legacy 需要单独的受控导入政策，不能伪造历史审批；
3. 对源目录建立只读、带哈希的完整备份；
4. 只允许注册表中的逐版本 `vN -> vN+1` 转换，不跳版本、不降级；
5. 严格解析旧 wire，非法 JSON、未知字段、重复键、坏哈希或不支持版本零写入失败；
6. 在新根/CAS 中 copy-on-write，完成全量 schema、跨聚合和完整性验证；
7. 用当前 DPAPI 上下文创建新的 signed revision 链和 anchor，不覆盖源链；
8. 输出 migration report，绑定源/备份/目标哈希、转换链、应用版本和人工 actor；
9. 只有全部检查通过才原子切换；失败保留源和备份，目标隔离；
10. 迁移后所有与内容或哈希不再一致的审批必须失效并重新人工审核。

在这些条件全部落地并有 kill-point 测试前，运维结论只能是 `NOT_MIGRATED`。

## 7. 不支持的“修复”方式

以下操作会破坏证据链或掩盖问题，禁止用于生产数据：

- 手工把 `schemaVersion` 改成当前值；
- 删除 `headMac`、key ID、signed sidecar 或 anchor；
- 用测试密钥重新计算 MAC；
- 复制旧 `head.json` 覆盖当前 head；
- 删除 head/anchor 后把某个历史 sidecar 冒充“待 roll-forward 提交”；
- 只恢复 CAS、不恢复完整项目头和 DPAPI 上下文；
- 调用 legacy `FsProjectStore::new` 绕过 secure constructor；
- 把导出的 `character.spine.json` 反向当作完整 ProjectManifest；
- 将别的机器/用户的 DPAPI 密文视为可移植明文密钥。

## 8. Schema 变更评审清单

每次评审至少核对：

- 字段所有权、命名、必填/可空和 `additionalProperties`；
- 规范化 JSON、SHA-256、Unicode、整数范围和 Windows 路径规则；
- ProjectManifest、ProjectHead、CAS、导出 snapshot 与历史记录的 revision 关系；
- 母版、分层、Rig、关键姿势素材、pose、hit 的审批失效传播；其中包括 Rig rest scale、slot bone/drawKey，以及 KeyPoseBinding revision/`groundYMilliPx`/`scalePpm`；
- HitFrame tick 是否受当前 MotionSpec `contact` phase 约束并精确引用当前 primary-weapon socket；
- Spine 4.2.43 常量、十动作顺序、三个攻击 hit 集合和单一主武器；
- Rust enum、TypeScript union、IPC schema 是否集合相等；
- 向前/向后 fixture、未知版本、篡改、回滚、分叉、进程中断和磁盘错误；
- 生成物是否由 `schemas/generate.mjs` 重建，而非手工编辑；
- 文档、诊断状态和 known limitations 是否同步，且没有把 `UNVERIFIED/EXTERNAL` 改写成 PASS。
