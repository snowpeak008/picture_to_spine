# F2S-FIXTURE-POLICY-001

`F2S-FIXTURE-SYNTHETIC-CHARACTER-001`只用于确定性测试图片导入、母版、分层、Rig IR、十动作、审批和导出报告链。它由本仓库生成器从几何图元创建，不包含用户或网络素材。

允许：单元/集成/E2E测试、golden hash、性能协议和错误注入。禁止：当作商业角色美术、训练数据代表性证明、真实目标域成功率或重度动作游戏质量承诺。所有动作关键姿势默认`UNAPPROVED_TEST_ONLY`，必须经过与真实素材相同的人工审批状态机。

任何源、生成器、许可或输出hash缺失/漂移时，整套fixture不得进入测试基线；不得只更新expected hash掩盖变化。
