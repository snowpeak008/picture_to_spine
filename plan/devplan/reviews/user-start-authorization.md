---
evidence_id: F2S-USER-START-AUTH-001
created_at_asia_shanghai: 2026-07-11T15:08:13+08:00
final_snapshot_id: FINAL-20260711-135705-R2
final_manifest_sha256: c3a0d964cbe34290619951dd1a8b9c32e8f06e228e9b234374611748ee76c9cc
final_audit_sha256: a46124e23c647afc64292d02e3d083ccb3fcb9c8baeb535905bf22a6b18a4f56
final_review_sha256: 4b791307403ed1e670dc67c7d74135a6d7d4f78af375622b8505e004437d291b
project_memory_sha256: 664de743a30e10a802f1c07a3cd62c6ab557ec69bfdf5386cc784ad82add7cfa
executionState: AUTHORIZED_FOR_IMPLEMENTATION
implementationAuthorized: true
releaseAuthorized: false
---

# 用户开始执行授权

## 1. 授权事实

用户在`FINAL-20260711-135705-R2`完成、detached final review裁决`PASS_DESIGN / WAITING_FOR_USER_START`之后，新发送了明确指令：

> 开始执行

该消息满足00、13、14、15与`PROJECT_MEMORY.md`定义的用户启动门。它只授权按照冻结原子计划创建源码、安装已审计依赖、运行测试和构建内部候选，不授权商业发布或伪造任何外部能力证据。

## 2. 当前状态

```text
executionState=AUTHORIZED_FOR_IMPLEMENTATION
implementationAuthorized=true
releaseAuthorized=false
```

## 3. 继续受限事项

- Spine Professional/适用Enterprise 4.2.43、真实Editor/CLI round-trip仍为用户外部能力；缺失时保持NOT_RUN/EXTERNAL。
- 私有远程GPU、代码签名证书、真实publisher/organization credential和法务意见不得伪造。
- `.atlas/.spine/.skel`不得由内置writer生成。
- 所有AI输出保持candidate，人工审批门不得绕过。
- 发布依赖继续执行宽松许可和供应链fail-closed门。

## 4. 执行入口

实施按`plan/devplan/13-原子任务DAG与追踪矩阵.md`从M00开始，逐DEV/WU生成真实EvidenceEnvelope；计划分数不能替代运行结果。
