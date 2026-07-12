# F2S-TOOLCHAIN-BASELINE-001

## 当前裁决

状态：`OBSERVED_LOCAL / UNVERIFIED_CLEAN_VM`

本地候选为 Node 24.15.0、npm 11.12.1、Rust/Cargo 1.96.0、Python 3.12.4、uv 0.11.8。MSVC、Windows SDK 与WebView2必须由`toolchain-probe.ps1`记录实际版本、来源和可用身份。

这些值只有在`clean-vm-a`和`clean-vm-b`两个独立Windows 11 x64 runner上使用相同来源重现，并且精确patch与二进制hash/签名身份一致后，才能升级为`VERIFIED`。当前机器观察不得替代该证据。

## 接受规则

- 禁止`latest`、版本范围和仅凭PATH文本输出判定成功。
- 每个可执行工具必须记录规范路径、精确版本、SHA-256、来源与退出码。
- Windows SDK和WebView2系统组件至少记录精确版本与安装来源；无法稳定取得二进制hash时保持UNVERIFIED。
- 任一patch差1、hash差1 bit、第二runner缺失或来源不一致均保持UNVERIFIED并阻断兼容性发布声明。
- 探测器不得安装、升级、卸载或修改系统工具链。

## M01消费边界

M01可以使用本地候选搭建内部开发骨架，但锁文件和发布基线必须保留`UNVERIFIED_CLEAN_VM`标识，直至双runner证据完成。
