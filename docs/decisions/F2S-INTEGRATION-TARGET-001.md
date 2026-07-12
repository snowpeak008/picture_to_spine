# F2S-INTEGRATION-TARGET-001

## 决策

V1唯一集成目标是Spine Editor/Professional CLI `4.2.43`。4.2.42、4.2.44、其他4.2 patch、`latest`和版本范围均不满足兼容声明。

## 内置开放输出

- Rig IR
- 最小分层PSD
- 透明attachment PNG
- Spine 4.2.43 JSON candidate
- `atlas-input-manifest.json`
- PromptPack与质量/证据报告

## 用户商业工具输出

`.atlas`、`.spine`、`.skel`只能由用户合法安装的Spine Professional或适用Enterprise 4.2.43 Editor/CLI在显式lease下生成。本产品不捆绑Editor、CLI、Runtime、激活信息或上述专有输出writer。

## 能力状态

静态开放格式契约通过不等于Editor兼容已验证。没有合法4.2.43工具和完整往返证据时，状态必须为`EXTERNAL/UNVERIFIED`。改变目标patch或增加游戏引擎集成必须建立新ADR、兼容矩阵和回归证据。
