# F2S-LICENSE-REVIEW-PLAYBOOK-001

## 新增或升级供应项

1. 先登记准确包名、精确版本、来源、SHA-256或锁文件身份、SPDX表达式、许可证文本证据、用途与是否进入安装包。
2. 将发布依赖、构建工具、测试资产/权重、系统运行时和用户商业工具分开；不得用名称猜许可证。
3. 生产依赖只接受`F2S-LIC-POLICY-001`中的MIT、Apache-2.0、BSD-2/3-Clause、ISC、Zlib、0BSD等宽松许可。
4. unknown、无证据、copyleft、source-available、NC、Research-only或来源不明项必须从Core/Worker发布图物理移除，不能用waiver改成warning。
5. 更新锁文件后离线运行`license-inventory-check.mjs`；锁、SBOM和inventory任一不一致即阻断release verify。

## 外部系统

Windows、WebView2、驱动、CUDA、Spine Editor/CLI和用户私有GPU是系统/用户外部能力，不作为开源依赖捆绑。Spine 4.2.43路径、许可确认和调用lease只保存在用户本地配置与证据中，不进入仓库。

## Core与Worker Pack

Core必须在没有Python、CUDA、模型、GPU和Spine时保持手工创建、编辑、保存、Rig IR预览和开放candidate导出。Worker Pack只有许可证和`windows-appcontainer-v1`完整控制均通过时才可进入对应内部候选；否则物理移除。

## 法务边界

本剧本不是法律意见。商业签名、Spine适用许可、训练数据与第三方资产等结论缺失时保持`EXTERNAL/UNVERIFIED`并阻断对应发布声明，但不伪造意见。
