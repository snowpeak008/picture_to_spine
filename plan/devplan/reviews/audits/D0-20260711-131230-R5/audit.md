# Devplan mechanical audit - D0-20260711-131230-R5

- evidenceId: F2S-AUD-DEVPLAN-D0-006
- phase: D0
- overallVerdict: FAIL
- checks: 11 PASS / 1 FAIL / 12 total
- implementationAuthorized: false
- releaseAuthorized: false

| Check | Status | Raw SHA-256 |
| --- | --- | --- |
| SNAPSHOT-INTEGRITY | PASS | 18c4323b0f9fb9058542993e48c71998313930029f955a37af2032c7abebbee9 |
| DOCUMENT-REGISTRY-FRONTMATTER | PASS | 394201fe4718fbb60edbaf66cecf7b2c8008f393871dc92cffc8b001d7721e4f |
| DOCUMENT-DEPENDENCY-DAG | PASS | 40331c1577cbb5f28bb9e2b60cc70f02be90cc9f7aafbe08628aeb44021c3870 |
| UPSTREAM-R3B-BINDING | PASS | 608e24af7f004ed1905ab016108e99cdd77e2bb33066b161c29bd1ae27bd3d6b |
| DEV-REGISTRY-PARITY | PASS | 0281c0119e824331924c7f4d8b04921294abe7d4fe629f1ba496b8fb1e2c6ab1 |
| TASK-CARD-STRUCTURE | PASS | ebb103b257cc7cff872593ef5304f46d1d7f4f61849d221da74af39bce7853c4 |
| WU-DAG-FIELDS-PATH-OWNERS | PASS | 971d05151b6ade9d18eea1620f3dc3fc7e3e941d2d8d22d1d010c919813af523 |
| NO-SHORTHAND-TRACE-IDS | PASS | 0e541665b65e6ac545a1a1f0a70256ce77993b930a02706fff5b7bbd207487a7 |
| REQUIREMENT-TEST-COVERAGE | PASS | 16cfabe9a743bc28091390fcba64bd02e3ecc1980e69ac6b1b2c8303b69e62f7 |
| R3B-P2-CARRYOVER | PASS | 29b06756c6297baf921d2a5a52189a3ffadb23c80dcabab8d03ff464a342ae0a |
| PRODUCT-SPINE-SECURITY-RELEASE-BOUNDARIES | PASS | dc7e894bf7c7040ed57bcefb43334625ce1ed4714e03d022fae859ba7c810f7f |
| TRACE-MATRIX-REVERSE-COVERAGE | FAIL | feee2eb6e90194faf94c5f6713ce9c2f7012b753552770cf81becebe876d1352 |

## Failures

- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 01-M00-决策与可行性Spike.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 02-M01-工程骨架与工具链.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 03-M02-领域存储与协议基线.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 04-M03-项目导入母版与审批.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 05-M04-分层与素材修复.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 06-M05-Rig编辑与审批.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 07-M06-动作内容与提示词.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 08-M07-动画编辑预览与玩法标记.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 09-M08-导出与Spine42适配.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 10-M09-安全AI远程GPU与质量.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 11-M10-Windows打包入口更新.md
- TRACE-MATRIX-REVERSE-COVERAGE: Trace source hash row missing: 12-M11-验收封账与发布候选.md
