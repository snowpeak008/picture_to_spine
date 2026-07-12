# F2S-M00-8GB-APPCONTAINER-001

## 当前裁决

状态：`UNVERIFIED`。Core不依赖GPU、Python、CUDA或Worker Pack；当前只完成只读身份/前置能力探测，不把本机存在GPU或系统DLL误写为安全能力通过。

## windows-appcontainer-v1硬门

商业D2 Worker必须在同一真实进程上同时证明：AppContainer token、空网络capability、专用Job root ACL、Job Object限制/kill-on-close、breakaway/句柄/子进程逃逸拒绝。五项中任一缺失或恶意探针成功，状态为FAILED并从发布图物理移除Worker Pack。普通进程、防火墙、仅ACL或策略声明不等价。

## 8GB矩阵

M09必须在用户授权测试机对512/1024/2048档记录精确GPU/显存/驱动/Python/CUDA身份、峰值VRAM/RAM、延迟、OOM、取消和缓存清理。低于8GB、无CUDA、driver不符、OOM或取消不得影响Core；只能禁用Worker或使用明确批准的CPU策略。

## 当前外部状态

探针不得安装驱动、Python、CUDA或模型，不上传fixture，不保存凭据。真实五控制与负载结果尚未运行，证据保持NOT_RUN/UNVERIFIED；这不阻断不含Worker的Core内部开发，但阻断Worker Pack发布。
