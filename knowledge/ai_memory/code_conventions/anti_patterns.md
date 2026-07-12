# 反模式与禁止事项

> 记录目标项目中已经确认不能做的事。

## 禁止提交

- 密钥、令牌、私有证书
- 本地机器路径配置
- 临时运行产物、缓存、日志

## 禁止改法

- 不得把目录名 `src-tauri` 当成仍在使用 Tauri/Wry 的证据，或重新引入未审计的 Tauri 插件。
- 不得让 UI 绕过 Rust 权威端直接写项目、CAS、审批或导出文件。
- 不得接受宽泛 `data:`/网络导航；内部 WebView 只允许精确的本轮嵌入文档。
- 不得通过删除 anchor、切换 legacy store、重签当前文件或忽略 HMAC 来“修复”项目。
- 不得内置生成 `.atlas/.spine/.skel`，也不得捆绑/下载/激活 Spine。
- 不得把 synthetic/mock/local 探针描述成真实 Spine、真实 GPU、clean-VM 或 Production Ready。
- 不得让构建、打包或更新脚本静默联网、提权、安装依赖或删除 `%LOCALAPPDATA%\FlashToSpine`。
- 不得用浮点秒作为领域权威时间；内部动画使用整数 tick。

## 历史踩坑

| 日期 | 问题 | 结论 |
|---|---|---|
| 2026-07-12 | `NavigationStarting` 只允许 `about:blank`，误拦截 `NavigateToString` 自己的 data URI，窗口存活但 UI 黑屏 | 白名单绑定本轮 HTML 的精确 data URI；导航完成并确认 React 根节点/`.app-shell` 后才发布正式标题，加入 Rust 与 Node 防回归测试 |
| 2026-07-12 | 仅检查进程、窗口和 Render 子窗口会把黑屏误判为启动成功 | 本机启动探针依赖 DOM 就绪后才出现的最终窗口标题，但仍明确不等于完整 GUI E2E |
