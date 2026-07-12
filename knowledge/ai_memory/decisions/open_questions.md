# 待解决问题

> 记录跨会话还没定下来的问题。每个问题都应有当前倾向或下一步动作。

## 产品/需求

- [ ] GUI 全业务链应采用何种可审计的人工/自动 E2E 方案覆盖原生图片选择、六类审批、关闭重开和导出复核？当前倾向先建立固定 fixture 与人工脚本，再评估 UI 自动化。

## 架构/工程

- [ ] 若启用私有远程 GPU，需先设计并评审真实 HTTPS/TLS/SPKI transport、一次性外传审批、quarantine 和删除状态语义；当前不得直接接入。
- [ ] 旧 unsigned/unanchored 项目是否需要 copy-on-write 迁移工具？当前保持 fail-closed，不以测试 store 绕过。

## 测试/发布

- [ ] 真实合法 Spine 4.2.43 CLI/Editor 往返、Editor 人工视觉验收何时具备外部资源？没有本轮 provenance 前保持 `NOT_RUN/EXTERNAL`。
- [ ] 何时安排两台 clean Windows 11 x64 runner、代码签名和组织 Release Gate？当前 `releaseAuthorized=false`。

## 记忆系统

- [ ] 会话结束记忆写入由用户明确要求触发，还是每次完成重要任务后自动触发？本次由用户明确要求，暂不推断长期默认策略。
