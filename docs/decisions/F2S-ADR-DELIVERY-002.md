# F2S-ADR-DELIVERY-002 — 以直接WebView2 Host替代Tauri/Vite候选

## 状态

`ACCEPTED_DURING_M01_LICENSE_GATE`，2026-07-11。

## 背景

M01首次锁定Tauri 2.11.5/Wry与Vite 8.1.4后，离线完整闭包审计发现Rust侧`cssparser`、`selectors`、`option-ext`等包和npm侧`lightningcss`平台包使用MPL-2.0。用户已冻结发布依赖只接受可审计的MIT、Apache、BSD等宽松许可，并明确禁止copyleft，因此该候选不能通过`F2S-LIC-POLICY-001`。

## 决策

- 移除Tauri、Wry、Vite、Vitest、LightningCSS及其MPL闭包。
- Rust Delivery改为MIT许可`webview2-com`加MIT/Apache许可`windows-rs`，直接托管系统WebView2 Evergreen Runtime。
- React/TypeScript/PixiJS保留；前端改用MIT许可esbuild生成固定`app.js/app.css`，并由Rust编译期嵌入同一exe。
- WebView文档使用严格CSP：无网络、无外部资源、无通用shell/fs/http。后续IPC只允许版本化白名单命令。
- Domain/Application/Adapter依赖方向、Rust权威revision、人工审批门和Rig IR边界保持不变。

## 后果

项目需要自行维护Win32/WebView2生命周期和白名单IPC，不能使用Tauri插件生态。换来的是MPL组件物理移除、更小的授权面，以及与用户许可约束一致的发布闭包。旧Tauri机械构建成功记录只作为失败候选证据，不进入正式包。
