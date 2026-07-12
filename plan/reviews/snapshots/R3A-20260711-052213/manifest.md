---
schemaVersion: 1
evidenceId: F2S-REV-R3A-PREFLIGHT-002
phase: R3A_PREFLIGHT_REJECTED
snapshotId: R3A-20260711-052213
createdAt: 2026-07-11T05:22:13+08:00
timezone: Asia/Shanghai
fileCount: 25
snapshotPath: plan/reviews/snapshots/R3A-20260711-052213/plan
archivePath: plan/reviews/snapshots/R3A-20260711-052213.zip
archiveSha256: ecfa9913f49dd828ff60db86f99090e9370556e6db587c68c90b44169e752731
rubricDocId: F2S-DOC-SCORE-001
rubricInputSha256: 0a601f7f01c94ff0be1a02081c0195aef2ad240483d7d21628a37fd89e9ab50a
status: REJECTED_BEFORE_SCORING
---

# R3a preflight-rejected snapshot manifest

The snapshot preserves actual candidate bytes but must not be scored or used as gate evidence. A deeper independent preflight found incomplete ApprovalRequest policy/finding binding and incomplete GateFinding hash determinism (`F2S-R3A-PRE-APPROVAL-001`). The live document was remediated afterward; a later snapshot must be used for R3a scoring. All copied files and the archive remain read-only, with live/copy SHA-256 equality originally verified for 25/25 files.

Snapshot tooling: Windows PowerShell `5.1.22621.4391`, executable `C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe`, SHA-256 `3247bcfd60f6dd25f34cb74b5889ab10ef1b3ec72b4d4b3d95b5b25b534560b8`. The exact operation was: enumerate and sort the 25 direct `plan/*.md` files; `Copy-Item -LiteralPath` each into the snapshot `plan/`; `Compress-Archive -LiteralPath <snapshot>/plan -CompressionLevel Optimal`; compare live/copy `Get-FileHash -Algorithm SHA256`; set copied files and ZIP read-only; hash the ZIP.

| File | doc_id | revision | bytes | SHA-256 |
| --- | --- | --- | ---: | --- |
| 00-项目索引与计划治理.md | F2S-DOC-GOV-001 | 1.6 | 16522 | bc13090f77a55ffcdd08240ce35883ce7b2f59dcea2dc48326140bd3aa0b9fdb |
| 01-产品章程范围与边界.md | F2S-DOC-CHARTER-001 | 1.5 | 18841 | 385bf266e7fea4cc6b47f15f9c2b0c8c827024493246e9bee5af0419e9f7bb8d |
| 02-需求规格与验收标准.md | F2S-DOC-REQ-001 | 1.5 | 32762 | 4c2f0f17c29fc0d28f4264e1192bc04e7219c9137e6169065479a07b7f4f62b9 |
| 03-内容动作素材与AI提示词设计.md | F2S-DOC-CONTENT-001 | 1.6 | 37218 | 178ad209dac8248bcc7682efc2f49305e56cc417063a844eb53b804385d1e9eb |
| 04-用户流程与信息架构.md | F2S-DOC-UX-001 | 1.7 | 25104 | 7fe49e16724882426e18b491d26013b5d7cdf66ea83e5d8c0b53cb9ce227bf1b |
| 05-UI设计系统与页面规格.md | F2S-DOC-UI-001 | 1.5 | 30705 | 08dfd63ef8a84f00b81dc25e22bdc4dfa0dad12ca713bdde40c4af81c078617b |
| 06-系统架构与设计模式.md | F2S-DOC-ARCH-001 | 1.5 | 17398 | 1a08b2e33b81d0a11307911b4f2e4e7ed5e5797408b0f3241118e640c9373f2b |
| 07-Windows环境配置与工具链.md | F2S-DOC-ENV-001 | 1.5 | 16752 | fc9f0e3f796c393b6d347a7feebe58b75d1b22ce54e54f2806d8f1e7c39a96d5 |
| 08-前端渲染与编辑器内核.md | F2S-DOC-RENDER-001 | 1.6 | 21430 | 5d8280faecaa2610b817bdae808d4a27502bee1abc9be691c128a33d85851114 |
| 09-领域逻辑与工作流编排.md | F2S-DOC-DOMAIN-001 | 1.8 | 29337 | 9f5bd01a21aeeecc50b66abe62908703179ec02793d641d69f56bd2ae4141ee1 |
| 10-本地图像处理与素材规划管线.md | F2S-DOC-PIPE-001 | 1.4 | 25629 | fadb5637fe8cefaccc52cc1e2fead928ddc650dc72254410272c8f9794b6a2e5 |
| 11-数据模型项目存储与迁移.md | F2S-DOC-STORE-001 | 1.8 | 30778 | c85a32b8841263ba8fab067b90bb057a64b4125b1306c66e886d8add4e406570 |
| 12-IPC协议后台Worker与远程GPU.md | F2S-DOC-IPC-001 | 1.7 | 20561 | 06c8ffba3df992a3b345988330a4547531da08a8080d62a17676617206605e82 |
| 13-RigIR-PSD-PNG-Spine42导出.md | F2S-DOC-EXPORT-001 | 1.8 | 30156 | 0347642ded10d86c516994d8524b6c5a6d540be0e003f36a5f9d259c4ae9dca0 |
| 14-安全隐私许可证与供应链.md | F2S-DOC-SEC-001 | 1.5 | 26746 | 4dad52cdc272fdd0e9004b94f70a011e40b45788b17778ea2a6385709420747e |
| 15-测试质量性能与可观测性.md | F2S-DOC-QUALITY-001 | 1.7 | 18492 | e7f335239586d4aa6011abbf27c3c0ebf1e19fc741c5ee7ab07361aa0418623c |
| 16-工程规范代码组织与协作.md | F2S-DOC-ENG-001 | 1.6 | 13768 | c1a985c2124954d51abae4001b18b713b46d955df3a55aee356471ec806afefa |
| 17-异常恢复兼容与项目升级.md | F2S-DOC-RECOVERY-001 | 1.7 | 23186 | 40bf177dce21b65ac54d8144f8d57d178e7ceb3bc3317c8801ef9ce6387a01ca |
| 18-交付发布安装更新与双击入口.md | F2S-DOC-RELEASE-001 | 1.5 | 25249 | fc8e7df004892a7eed67730dc4e00752f4d51de4298ee404d4bf74fa84247789 |
| 19-风险登记与应对.md | F2S-DOC-RISK-001 | 1.5 | 13578 | acca796df9d247e93d47ac5ba00dff4597af0a7c8f521074a4d9099486f4120e |
| 20-路线图里程碑依赖与估算.md | F2S-DOC-ROADMAP-001 | 1.5 | 12582 | 9cc811049a525613610a0b428fd488b0e2718b29a15af45717e0307f025be833 |
| 21-需求设计任务测试追踪矩阵.md | F2S-DOC-TRACE-001 | 2.0 | 49479 | 34e82423214f7f267ad56cdcafea76af370a7e6ffac0cd9c3979bbb37d030272 |
| 22-逐文件评分与统筹整改记录.md | F2S-DOC-SCORE-001 | 1.8 | 22969 | 0a601f7f01c94ff0be1a02081c0195aef2ad240483d7d21628a37fd89e9ab50a |
| 23-最终统筹合规审查.md | F2S-DOC-COMPLIANCE-001 | 1.6 | 12407 | feac4150dafc80904b77cf288851127f90f325915f8216bcdcfc9d6430697942 |
| 24-架构决策ADR与未决事项.md | F2S-DOC-ADR-001 | 1.6 | 28403 | 6f22ff36e0b2cda203dc88d09fe6ffa0151855f900561e42dace1517ee95cb88 |
